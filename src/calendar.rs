//! Calendar core: the range lunar-rs can serve, solar/lunar conversion,
//! ganzhi, festivals and solar terms.
//!
//! Every calendar computation is delegated to the `lunar-rs` crate (ShouXing
//! astronomical engine). This module only shapes its output for the CLI.

use std::fmt;
use std::sync::Arc;

use lunar_rs::solar_util;
use lunar_rs::{Holiday, Lunar, LunarFestival, LunarMonth, LunarYear, Solar};

use crate::civil::CivilDate;

/// Smallest civil year the CLI accepts.
///
/// `lunar-rs` is engineered and documented for 公元 1–9999: its ShouXing
/// astronomy is constrained by the `LEAP_11` / `LEAP_12` tables, so outside
/// this window its extrapolated lunar months and solar terms are not meant to
/// be relied on. This is the widest range lunar-rs can actually serve.
pub const MIN_YEAR: i32 = 1;

/// Largest civil year the CLI accepts (see [`MIN_YEAR`]).
pub const MAX_YEAR: i32 = 9999;

/// Everything that can go wrong while resolving a date or a month.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalError {
    /// A year outside the supported window was requested.
    ///
    /// `year` is the year **as computed**, which need not fit an `i32`: a
    /// relative count the user wrote can carry a valid year out of the
    /// representable range, and naming the year that was reached is the
    /// honest answer — a wrapped one would be a year nobody asked for.
    YearOutOfRange { year: i64, min: i32, max: i32 },
    /// A year that the Gregorian calendar itself skips (1582: 10-05..=10-14).
    YearMissing { year: i32 },
    /// A month number outside `1..=12` (lunar months may be negative: `-4` is
    /// the leap fourth month).
    MonthOutOfRange { month: i32 },
    /// A day number outside `1..=31`.
    DayOutOfRange { day: i32 },
    /// The requested civil date does not exist, e.g. 2023-02-30.
    NonexistentDate { year: i32, month: i32, day: i32 },
    /// A lunar year that has no such month.
    NoSuchLunarMonth { year: i32, month: i32 },
    /// The selected lunar year has no leap month while `-R` was requested.
    NoLeapMonth { year: i32 },
    /// A `-d` string could not be parsed.
    UnparsableDate { input: String },
    /// `TZ` names a zone `tz-rs` cannot parse, or the system zone is
    /// unreadable. Carries which of the two it was.
    BadTimeZone { source: &'static str },
    /// The system clock reads before the Unix epoch.
    ClockBeforeEpoch,
    /// A lunar day that does not exist in the requested lunar month.
    LunarDayOutOfRange {
        year: i32,
        month: i32,
        day: i32,
        max: i32,
    },
    /// A lunar month number outside `1..=12`, the leap form included
    /// (`-4` is the leap fourth month), or zero.
    LunarMonthOutOfRange { month: i32 },
    /// A lunar month whose days cross the supported civil range.
    ///
    /// 腊月 of the last supported lunar year begins in December 9999 and
    /// ends in January 10000, so a grid — which must draw every day the
    /// month *has* — reaches past the end of the window. The month itself is
    /// one the engine has and the reader named correctly; it is the days it
    /// runs through that leave the range, and a bare 年份 10000 reads as a
    /// year somebody typed.
    LunarMonthBeyondRange {
        lunar_year: i32,
        month: i32,
        civil_year: i32,
    },
    /// A relative offset shorter than a day, which cannot move a date-only
    /// answer. Carries the unit as written, e.g. `minutes`.
    SubDayUnit { unit: String },
    /// A positional argument that is not a number, or one too many of them.
    /// Carries the offending text, empty when the count is what is wrong.
    BadArgument { detail: String },
}

impl CalError {
    /// The user-facing message, in Simplified Chinese.
    pub fn message(&self) -> String {
        match self {
            Self::YearOutOfRange { year, min, max } => {
                format!("年份 {year} 超出支持范围 ({min}–{max})")
            }
            Self::YearMissing { year } => {
                format!("{year} 年 10 月 5 日至 14 日不存在（公历改革跳过的 10 天）")
            }
            Self::MonthOutOfRange { month } => format!("月份 {month} 非法 (应为 1–12)"),
            Self::DayOutOfRange { day } => format!("日期 {day} 非法 (应为 1–31)"),
            Self::NonexistentDate { year, month, day } => {
                format!("公历 {year}-{month:02}-{day:02} 不存在")
            }
            Self::NoSuchLunarMonth { year, month } => {
                format!("农历 {year} 年没有{}", m_abs(*month))
            }
            Self::NoLeapMonth { year } => format!("农历 {year} 年没有闰月"),
            Self::LunarDayOutOfRange {
                year,
                month,
                day,
                max,
            } => {
                format!(
                    "农历 {year} 年{}没有第 {day} 天 (该月只有 {max} 天)",
                    m_abs(*month)
                )
            }
            Self::LunarMonthOutOfRange { month } => {
                format!("农历月份 {month} 非法 (应为 1–12，闰月为负数)")
            }
            Self::LunarMonthBeyondRange {
                lunar_year,
                month,
                civil_year,
            } => format!(
                "农历 {lunar_year} 年{}跨入 {civil_year} 年，超出支持范围 ({MIN_YEAR}–{MAX_YEAR})",
                m_abs(*month)
            ),
            Self::SubDayUnit { unit } => {
                format!("日期偏移单位 {unit} 不足一天，本工具只输出日期")
            }
            Self::UnparsableDate { input } => format!("无法解析的日期: {input}"),
            Self::BadTimeZone { source } => format!("无法读取时区: {source}"),
            Self::ClockBeforeEpoch => "系统时钟早于 1970-01-01".to_string(),
            Self::BadArgument { detail } => match detail.is_empty() {
                true => "位置参数过多".to_string(),
                false => format!("无法解析的位置参数: {detail}"),
            },
        }
    }
}

impl fmt::Display for CalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message())
    }
}

impl std::error::Error for CalError {}

/// Renders a possibly negative (leap) lunar month number in Chinese.
///
/// `month` here is **user input** — it reaches the error arms of
/// [`CalError::message`] — so it is range-checked before it indexes
/// `lunar_util::MONTH`; a number outside `1..=12` has no month name, and the
/// raw number is the honest thing to print. The leap prefix is the same rule
/// `Lunar::month_in_chinese` applies, and is used unchanged for the leap form
/// a lunar year can actually have.
fn m_abs(month: i32) -> String {
    match check_lunar_month(month) {
        Ok(()) => format!(
            "{}{}月",
            if month < 0 { "闰" } else { "" },
            lunar_rs::lunar_util::MONTH[month.unsigned_abs() as usize]
        ),
        Err(_) => month.to_string(),
    }
}

/// Whether a civil year falls inside the supported window.
fn in_range(year: i32) -> bool {
    (MIN_YEAR..=MAX_YEAR).contains(&year)
}

/// Checks a civil year against the supported window.
pub fn check_year(year: i32) -> Result<(), CalError> {
    if in_range(year) {
        Ok(())
    } else {
        Err(CalError::YearOutOfRange {
            year: i64::from(year),
            min: MIN_YEAR,
            max: MAX_YEAR,
        })
    }
}

/// Checks a civil month (`1..=12`).
pub fn check_month(month: i32) -> Result<(), CalError> {
    if (1..=12).contains(&month) {
        Ok(())
    } else {
        Err(CalError::MonthOutOfRange { month })
    }
}

/// Checks a civil day (`1..=31`).
pub fn check_day(day: i32) -> Result<(), CalError> {
    if (1..=31).contains(&day) {
        Ok(())
    } else {
        Err(CalError::DayOutOfRange { day })
    }
}

/// Builds a [`Solar`] after range and existence checks.
pub fn solar(year: i32, month: i32, day: i32) -> Result<Solar, CalError> {
    check_year(year)?;
    check_month(month)?;
    check_day(day)?;
    Solar::from_ymd(year, month, day).map_err(|error| match error {
        // lunar-rs models the 1582 Gregorian reform, so 1582-10-05..=14 are
        // the one civil range it refuses.
        lunar_rs::LunarError::GregorianGap { .. } => CalError::YearMissing { year },
        _ => CalError::NonexistentDate { year, month, day },
    })
}

/// Whether `month` is a lunar month number: `1..=12`, a negative number
/// standing for the leap month of that ordinal.
pub fn check_lunar_month(month: i32) -> Result<(), CalError> {
    match month.checked_abs() {
        Some(1..=12) => Ok(()),
        _ => Err(CalError::LunarMonthOutOfRange { month }),
    }
}

/// The civil day a lunar year / month / day falls on.
///
/// The result is checked by **round trip** — the day is converted back and
/// must be the very year / month / day that was asked for — rather than by
/// comparing *civil* years. A lunar year and a civil year do not line up:
/// 腊月 always begins in the following January, so 农历 2026 年腊月初一 is
/// 公历 2027-01-08. The earlier civil-year comparison rejected every such
/// day (336,947 of the 3,652,046 days in 1–9999), making 腊月 unreachable;
/// 闰腊月 and two anomalous 正月 were refused the same way. The round trip is
/// exact: over the same range it accepts all 3,652,046 days and rejects
/// none, because it compares the whole triple instead of one field of it.
///
/// A month absent from that year is [`CalError::NoSuchLunarMonth`] (a leap
/// month asked for without `-R` being meaningful, an ordinary one simply out
/// of range) and a day past the end of the month is
/// [`CalError::LunarDayOutOfRange`].
pub fn solar_from_lunar(year: i32, month: i32, day: i32) -> Result<Solar, CalError> {
    check_year(year)?;
    check_lunar_month(month)?;
    let lunar_year = LunarYear::from_year(year);
    let Some(lunar_month) = lunar_year_month(&lunar_year, month) else {
        return Err(CalError::NoSuchLunarMonth { year, month });
    };
    let max = lunar_month.get_day_count();
    if day < 1 || day > max {
        return Err(CalError::LunarDayOutOfRange {
            year,
            month,
            day,
            max,
        });
    }
    let lunar = Lunar::from_ymd(year, month, day).map_err(|_| CalError::LunarDayOutOfRange {
        year,
        month,
        day,
        max,
    })?;
    let solar = lunar.solar();
    let back = solar.lunar();
    match (back.year(), back.month(), back.day()) == (year, month, day) {
        true => Ok(solar),
        false => Err(CalError::NoSuchLunarMonth { year, month }),
    }
}

/// Chinese lunar month name: `正月`, `闰四月`, `腊月`.
///
/// Shares [`m_abs`] with the error messages, which is deliberate: one
/// rendering of a month number, and one place where it is range-checked. The
/// leap prefix follows the same rule `Lunar::month_in_chinese` applies, and
/// the 月 suffix is this tool's — the engine's own name stops at 正 or 闰四.
pub fn lunar_month_name(month: i32) -> String {
    m_abs(month)
}

/// Chinese lunar day name: `初一` … `三十`.
pub fn lunar_day_name(day: i32) -> &'static str {
    lunar_rs::lunar_util::DAY[day as usize]
}

/// Weekday label: `日` … `六`.
pub fn weekday_name(weekday: i32) -> &'static str {
    solar_util::WEEK[weekday as usize]
}

/// Number of days in a civil month, honouring lunar-rs's 1582 reform model.
pub fn days_in_civil_month(year: i32, month: i32) -> i32 {
    solar_util::days_of_month(year, month)
}

/// GanZhi of the **lunar** year: `丙午` — the 春节 basis.
///
/// A lunar year turns at 春节, not at 立春, so this is the pillar that
/// names the 农历 year itself, and it is what the `农历:` line, `%G`, the
/// 生肖 and the `cal` lunar titles all report. The 干支 chain uses the
/// other basis; see [`li_chun_year_gan_zhi`].
pub fn year_gan_zhi(lunar: &Lunar) -> String {
    lunar.year_in_gan_zhi()
}

/// GanZhi of the lunar **month** pillar, as the 黄历 numbers it.
///
/// This is the day-granular pillar: the month turns at the 節氣 *day*, so
/// the whole of that day already belongs to the new month. It is also the
/// pillar `Lunar::day_yi` and `Lunar::day_ji` key their 宜 / 忌 tables on
/// (`day_yi_by_sect(1)`), so printing it is what keeps the `干支` line and
/// the `宜:` / `忌:` lines describing the same day in one system.
///
/// The engine also offers `month_in_gan_zhi_exact()`, which turns at the
/// 節氣 *instant* and therefore disagrees on each of the twelve 節氣 days.
/// Both are correct answers to different questions; the almanac takes the
/// day-granular one, and so does this wrapper.
pub fn month_gan_zhi(lunar: &Lunar) -> String {
    lunar.month_in_gan_zhi()
}

/// GanZhi of the year pillar as the `干支` line numbers it: the 立春 basis.
///
/// The month pillar counts from 立春, so the month stem that goes with it
/// is the one 五虎遁 derives from the *立春* year stem; a 春节 year stem
/// paired with a 立春 month stem is a pair the two pillars can never form.
/// Over 公元 1–9999 this basis is self-consistent on every day, which the
/// 春节 basis is not for 74,948 of them.
///
/// [`year_gan_zhi`] is the other basis, and it is what the `农历:` line, the
/// `%G` token and the `cal` lunar titles use: a lunar year turns at 春节,
/// and those three name the lunar year itself rather than the 干支 chain.
pub fn li_chun_year_gan_zhi(lunar: &Lunar) -> String {
    lunar.year_in_gan_zhi_by_li_chun()
}

/// GanZhi of the day pillar: `甲申`.
pub fn day_gan_zhi(lunar: &Lunar) -> String {
    lunar.day_in_gan_zhi()
}

/// Chinese zodiac animal of the lunar year.
pub fn sheng_xiao(lunar: &Lunar) -> String {
    lunar.year_sheng_xiao().to_string()
}

/// The solar term falling exactly on this day, if any.
pub fn jie_qi(lunar: &Lunar) -> Option<String> {
    match lunar.jie_qi() {
        "" => None,
        name => Some(name.to_string()),
    }
}

/// The solar-sign / constellation of a civil day: `天秤` and the other eleven.
///
/// A function of the civil month and day alone, which is why it is the one
/// almanac field the default profile can always print: every day of
/// 1–9999 has one.
pub fn xing_zuo(solar: &Solar) -> &'static str {
    solar.xing_zuo()
}

/// How many blank cells precede day 1 of a civil month when a week starts on
/// `week_start` (0 = Sunday).
pub fn week_offset(year: i32, month: i32, week_start: i32) -> usize {
    (solar_util::week(year, month, 1) - week_start).rem_euclid(7) as usize
}

/// The festival a cell shows, or `None` when the day carries none.
///
/// Three sources are consulted, and all three are needed. `Solar::festivals`
/// carries the civil ones (国庆节, 劳动节) plus the weekday-floating ones
/// (母亲节, 感恩节). The lunar half has to come from the **typed** lookup,
/// `LunarFestival::from_ymd`, not from `Lunar::festivals`: the latter is
/// driven by a different table (`lunar_util::FESTIVAL_INDEX`) which omits
/// 清明节, 上巳节, 中元节 and 冬至节 entirely. Reading only the string list
/// made all four unreachable — a 七月十五 cell read 十五, never 中元节.
///
/// The name is the engine's own, `中秋节` and not a shortened `中秋`; when a
/// day carries several, the one a reader recognises wins.
pub fn traditional_festivals(solar: &Solar, lunar: &Lunar) -> Option<String> {
    let lunar_name = LunarFestival::from_ymd(lunar.year(), lunar.month(), lunar.day());
    solar
        .festivals()
        .iter()
        .chain(lunar.festivals().iter())
        .copied()
        .chain(lunar_name.map(|festival| festival.name()))
        .min_by_key(|name| festival_rank(name))
        .map(|name| name.to_string())
}

/// Sort key for [`traditional_festivals`]: lower wins.
///
/// The festivals a reader scans a calendar for come first, lunar and civil
/// alike; everything else follows, so a busy day still shows its most
/// recognisable name.
fn festival_rank(name: &str) -> usize {
    const PRINCIPAL: [&str; 20] = [
        "春节",
        "元宵节",
        "清明节",
        "端午节",
        "七夕节",
        "中元节",
        "中秋节",
        "重阳节",
        "除夕",
        "上巳节",
        "冬至节",
        "元旦节",
        "劳动节",
        "国庆节",
        "儿童节",
        "青年节",
        "妇女节",
        "教师节",
        "腊八节",
        "龙头节",
    ];
    PRINCIPAL
        .iter()
        .position(|principal| *principal == name)
        .unwrap_or(usize::MAX)
}

/// The statutory calendar entry for a civil day, if it has one.
///
/// This is the 法定节假日 table the State Council publishes each year: a day
/// off (`is_work` false) and the 调休 workdays it moves onto weekends
/// (`is_work` true). It is *not* the traditional festival list —
/// [`traditional_festivals`] already has that — and it is not derived from the
/// weekday either, since a 调休 Saturday is exactly the day the two disagree.
/// The engine only carries the years it was given, so a day outside them has
/// no entry and is drawn as an ordinary day.
pub fn legal_holiday(solar: &Solar) -> Option<Holiday> {
    lunar_rs::holiday_util::get_holiday_by_ymd(solar.year(), solar.month(), solar.day())
}

/// One `法定: …` line of the `date` profile, or `None` for an ordinary day.
///
/// The line carries both halves of the statutory entry — the name and
/// whether the day is 放假 or 调休上班 — because that is the one thing a
/// reader of a day profile has no other way to learn, and the 调休 half in
/// particular contradicts the weekday the profile prints above it.
pub fn legal_holiday_line(solar: &Solar) -> Option<String> {
    let holiday = legal_holiday(solar)?;
    Some(format!(
        "法定: {}{}",
        holiday.get_name(),
        match holiday.is_work() {
            true => " 调休上班",
            false => " 放假",
        }
    ))
}

/// A whole lunar year, for `cal -L <year>`.
pub fn lunar_year(year: i32) -> Result<Arc<LunarYear>, CalError> {
    check_year(year)?;
    Ok(LunarYear::from_year(year))
}
/// First civil day of a lunar month.
pub fn lunar_month_start(month: &LunarMonth) -> CivilDate {
    let first = month.first_solar_day();
    CivilDate::new(first.year(), first.month(), first.day())
}

/// Every civil day of a lunar month, in order, or the range error its days
/// run into.
///
/// Asked of the engine rather than stepped locally: the engine's calendar is
/// not the proleptic one before 1600 and it skips the ten reform days of
/// October 1582, so a local `add_days` walk drifts a day per Julian February
/// and walks straight into the gap. A grid has to draw the days the month
/// *has*, and since a cell's content is derived from the same lookup, the
/// date band and the content band cannot disagree.
///
/// The month is not clipped to fit the window, because a clipped grid and
/// `lunar date` would then answer differently about the same day. 农历
/// 9999 年腊月 fails as a whole instead — see
/// [`CalError::LunarMonthBeyondRange`].
pub fn lunar_month_days(month: &LunarMonth) -> Result<Vec<CivilDate>, CalError> {
    let days = month.get_days();
    let mut out = Vec::with_capacity(days.len());
    for lunar in &days {
        let solar = lunar.solar();
        if !in_range(solar.year()) {
            return Err(CalError::LunarMonthBeyondRange {
                lunar_year: month.year(),
                month: month.month(),
                civil_year: solar.year(),
            });
        }
        out.push(CivilDate::new(solar.year(), solar.month(), solar.day()));
    }
    Ok(out)
}

/// Looks up one month of a lunar year by its number (`-4` is the leap fourth
/// month), or `None` when the year has no such month.
pub fn lunar_year_month(year: &LunarYear, month: i32) -> Option<LunarMonth> {
    year.get_month(month)
}

/// The months of a lunar year that belong to it, in calendar order: 正月
/// through 冬月/腊月, with the leap month in its place.
pub fn lunar_year_months(year: &LunarYear) -> Vec<LunarMonth> {
    year.months_in_year().collect()
}

/// The 黄历 block for one day, as `label: text` lines.
///
/// Every line is a one-line forward to the engine's day-level almanac: the
/// 宜 / 忌 tables, 冲煞, 值神, 十二神煞, 吉神 / 凶煞, 二十八宿, 纳音, 旬空,
/// 六曜, 小六壬, 九星, 月相, 彭祖百忌, 胎神, the five 方位 and the 物候.
/// None of them depends on a time of day, which is what lets a date-only tool
/// print them at all — the ones that would (`time_yi`, `time_chong`) are not
/// here. The two that can be absent on a given day — 数九 (冬至后 81 天) and
/// 三伏 (夏至后 30 天起) — are dropped rather than printed empty, the same
/// way the `节气` line of the day profile is.
///
/// The 十二神煞 and the 天神 are the engine's `get_twelve_star` and
/// `day_tian_shen`; they differ only on a solar-term day (12 a year), and the
/// almanac prints the 黄道 / 黑道 type, which is what a reader acts on.
pub fn almanac(lunar: &Lunar) -> Vec<String> {
    let mut lines = Vec::with_capacity(18);
    let mut line = |label: &str, text: String| {
        if !text.is_empty() {
            lines.push(format!("{label}: {text}"));
        }
    };

    line("宜", lunar.day_yi().join("、"));
    line("忌", lunar.day_ji().join("、"));
    line(
        "冲煞",
        format!("{} 煞{}", lunar.day_chong_desc(), lunar.day_sha()),
    );
    line(
        "值神",
        format!(
            "{}  十二神煞: {}({})",
            lunar.zhi_xing(),
            lunar.day_tian_shen(),
            lunar.day_tian_shen_type()
        ),
    );
    line(
        "吉神",
        format!(
            "{}  凶煞: {}",
            lunar.day_ji_shen().join("、"),
            lunar.day_xiong_sha().join("、")
        ),
    );
    line(
        "二十八宿",
        format!(
            "{}({}) 值{} 禽{} 门{}{}",
            lunar.xiu(),
            lunar.xiu_luck(),
            lunar.zheng(),
            lunar.animal(),
            lunar.gong(),
            lunar.shou()
        ),
    );
    line(
        "纳音",
        format!(
            "{}  旬空: {}  六曜: {}  小六壬: {}  九星: {}  月相: {}",
            lunar.day_nayin(),
            lunar.day_xun_kong(),
            lunar.liu_yao(),
            lunar.day_minor_ren().name(),
            lunar.day_nine_star(),
            lunar.yue_xiang()
        ),
    );
    line(
        "彭祖百忌",
        format!("{} / {}", lunar.peng_zu_gan(), lunar.peng_zu_zhi()),
    );
    line(
        "胎神",
        format!(
            "{}  胎元: {}",
            lunar.day_position_tai(),
            lunar.day_position_tai_sui()
        ),
    );
    line(
        "方位",
        format!(
            "喜神{}({})、阳贵{}({})、阴贵{}({})、福神{}({})、财神{}({})",
            lunar.day_position_xi(),
            lunar.day_position_xi_desc(),
            lunar.day_position_yang_gui(),
            lunar.day_position_yang_gui_desc(),
            lunar.day_position_yin_gui(),
            lunar.day_position_yin_gui_desc(),
            lunar.day_position_fu(),
            lunar.day_position_fu_desc(),
            lunar.day_position_cai(),
            lunar.day_position_cai_desc()
        ),
    );
    line("物候", format!("{} ({})", lunar.wu_hou(), lunar.hou()));
    if let Some(shu_jiu) = lunar.shu_jiu() {
        line("数九", format!("{shu_jiu}"));
    }
    if let Some(fu) = lunar.fu() {
        line("三伏", format!("{fu}"));
    }
    lines
}
