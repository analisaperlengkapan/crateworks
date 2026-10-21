//! Solar position, after Jean Meeus, "Astronomical Algorithms".
//!
//! Only the terms needed for prayer times are kept. Every function here is
//! checked against the reference values the `adhan` library produces, so the
//! names follow its vocabulary: `transit` is solar noon, `hour_angle` is the
//! time offset from noon to a given solar altitude.

/// Degrees to radians.
pub(crate) fn to_radians(degrees: f64) -> f64 {
    degrees * std::f64::consts::PI / 180.0
}

/// Radians to degrees.
pub(crate) fn to_degrees(radians: f64) -> f64 {
    radians * 180.0 / std::f64::consts::PI
}

/// Reduces an angle to `[0, 360)`.
pub(crate) fn unwind_angle(angle: f64) -> f64 {
    angle.rem_euclid(360.0)
}

/// Reduces an angle to `[-180, 180]`.
///
/// Used where the sign carries meaning, such as a local hour angle that can be
/// either side of the meridian.
pub(crate) fn quadrant_shift(angle: f64) -> f64 {
    if (-180.0..=180.0).contains(&angle) {
        angle
    } else {
        angle - 360.0 * (angle / 360.0).round()
    }
}

/// Reduces a fraction to `[0, max)`.
fn normalize_to_scale(value: f64, max: f64) -> f64 {
    value - max * (value / max).floor()
}

/// The Julian day number at midnight UTC for a Gregorian date.
///
/// Meeus, chapter 7. The Julian day starts at noon, which is why the result
/// carries the half day.
pub(crate) fn julian_day(year: i32, month: u32, day: u32) -> f64 {
    let year = if month > 2 { year } else { year - 1 } as f64;
    let month = if month > 2 { month } else { month + 12 } as f64;
    let day = day as f64;

    let a = (year / 100.0).trunc();
    let b = 2.0 - a + (a / 4.0).trunc();
    (365.25 * (year + 4716.0)).trunc() + (30.6001 * (month + 1.0)).trunc() + day + b - 1524.5
}

/// Julian centuries since J2000.
pub(crate) fn julian_century(julian_day: f64) -> f64 {
    (julian_day - 2_451_545.0) / 36_525.0
}

/// The sun's geometric mean longitude, in degrees.
fn mean_solar_longitude(t: f64) -> f64 {
    unwind_angle(280.466_456_7 + 36_000.769_83 * t + 0.000_303_2 * t * t)
}

/// The moon's geometric mean longitude, in degrees.
///
/// The moon appears here only because its longitude drives the nutation terms.
fn mean_lunar_longitude(t: f64) -> f64 {
    unwind_angle(218.316_5 + 481_267.881_3 * t)
}

/// The longitude of the moon's ascending node, in degrees.
fn ascending_lunar_node(t: f64) -> f64 {
    unwind_angle(125.044_52 - 1_934.136_261 * t + 0.002_070_8 * t * t + t * t * t / 450_000.0)
}

/// The sun's mean anomaly, in degrees.
fn mean_solar_anomaly(t: f64) -> f64 {
    unwind_angle(357.529_11 + 35_999.050_29 * t - 0.000_153_7 * t * t)
}

/// The sun's equation of the centre, in degrees.
fn solar_equation_of_the_centre(t: f64) -> f64 {
    let m = to_radians(mean_solar_anomaly(t));
    (1.914_602 - 0.004_817 * t - 0.000_014 * t * t) * m.sin()
        + (0.019_993 - 0.000_101 * t) * (2.0 * m).sin()
        + 0.000_289 * (3.0 * m).sin()
}

/// The sun's apparent longitude, in degrees.
fn apparent_solar_longitude(t: f64) -> f64 {
    let longitude = mean_solar_longitude(t) + solar_equation_of_the_centre(t);
    let omega = 125.04 - 1_934.136 * t;
    unwind_angle(longitude - 0.005_69 - 0.004_78 * to_radians(omega).sin())
}

/// The mean obliquity of the ecliptic, in degrees.
fn mean_obliquity_of_the_ecliptic(t: f64) -> f64 {
    23.439_291 - 0.013_004_167 * t - 0.000_000_163_9 * t * t + 0.000_000_503_6 * t * t * t
}

/// The obliquity corrected for the sun's apparent position, in degrees.
fn apparent_obliquity_of_the_ecliptic(t: f64, mean: f64) -> f64 {
    let omega = 125.04 - 1_934.136 * t;
    mean + 0.002_56 * to_radians(omega).cos()
}

/// Mean sidereal time, the hour angle of the vernal equinox, in degrees.
fn mean_sidereal_time(t: f64) -> f64 {
    let jd = t * 36_525.0 + 2_451_545.0;
    let theta = 280.460_618_37 + 360.985_647_366_29 * (jd - 2_451_545.0) + 0.000_387_933 * t * t
        - t * t * t / 38_710_000.0;
    unwind_angle(theta)
}

/// Nutation in longitude, in degrees.
fn nutation_in_longitude(_t: f64, solar: f64, lunar: f64, node: f64) -> f64 {
    -17.2 / 3600.0 * to_radians(node).sin()
        - 1.32 / 3600.0 * to_radians(2.0 * solar).sin()
        - 0.23 / 3600.0 * to_radians(2.0 * lunar).sin()
        + 0.21 / 3600.0 * to_radians(2.0 * node).sin()
}

/// Nutation in obliquity, in degrees.
fn nutation_in_obliquity(_t: f64, solar: f64, lunar: f64, node: f64) -> f64 {
    9.2 / 3600.0 * to_radians(node).cos()
        + 0.57 / 3600.0 * to_radians(2.0 * solar).cos()
        + 0.1 / 3600.0 * to_radians(2.0 * lunar).cos()
        - 0.09 / 3600.0 * to_radians(2.0 * node).cos()
}

/// The sun's declination and right ascension, and the apparent sidereal time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SolarCoordinates {
    /// The angle between the sun's rays and the equatorial plane, in degrees.
    pub(crate) declination: f64,
    /// The sun's angular distance along the celestial equator, in degrees.
    pub(crate) right_ascension: f64,
    /// The hour angle of the vernal equinox, in degrees.
    pub(crate) apparent_sidereal_time: f64,
}

impl SolarCoordinates {
    /// Computes the sun's position at a Julian day.
    pub(crate) fn new(julian_day: f64) -> Self {
        let t = julian_century(julian_day);
        let l0 = mean_solar_longitude(t);
        let lp = mean_lunar_longitude(t);
        let omega = ascending_lunar_node(t);
        let lambda = to_radians(apparent_solar_longitude(t));
        let theta0 = mean_sidereal_time(t);
        let d_psi = nutation_in_longitude(t, l0, lp, omega);
        let d_epsilon = nutation_in_obliquity(t, l0, lp, omega);
        let epsilon0 = mean_obliquity_of_the_ecliptic(t);
        let epsilon = to_radians(apparent_obliquity_of_the_ecliptic(t, epsilon0));

        Self {
            declination: to_degrees((epsilon.sin() * lambda.sin()).asin()),
            right_ascension: unwind_angle(to_degrees(
                (epsilon.cos() * lambda.sin()).atan2(lambda.cos()),
            )),
            apparent_sidereal_time: theta0
                + d_psi * 3600.0 * to_radians(epsilon0 + d_epsilon).cos() / 3600.0,
        }
    }
}

/// Interpolates a value from its neighbours, Meeus chapter 3.
fn interpolate(y2: f64, y1: f64, y3: f64, n: f64) -> f64 {
    let a = y2 - y1;
    let b = y3 - y2;
    let c = b - a;
    y2 + n / 2.0 * (a + b + n * c)
}

/// Interpolates three angles, unwinding each difference first.
fn interpolate_angles(y2: f64, y1: f64, y3: f64, n: f64) -> f64 {
    let a = unwind_angle(y2 - y1);
    let b = unwind_angle(y3 - y2);
    let c = b - a;
    y2 + n / 2.0 * (a + b + n * c)
}

/// The sun's altitude above the horizon, in degrees.
fn altitude_of(latitude: f64, declination: f64, hour_angle: f64) -> f64 {
    let phi = to_radians(latitude);
    let delta = to_radians(declination);
    let h = to_radians(hour_angle);
    to_degrees((phi.sin() * delta.sin() + phi.cos() * delta.cos() * h.cos()).asin())
}

/// The fraction of a day at which the sun crosses the meridian.
///
/// The first guess uses the longitude alone. Near the international date line
/// that guess can land on the wrong calendar day, so it is compared against the
/// longitude-based estimate and moved a whole day when the two disagree by more
/// than half a day.
fn approximate_transit(longitude: f64, sidereal_time: f64, right_ascension: f64) -> f64 {
    let m0 = normalize_to_scale((right_ascension - longitude - sidereal_time) / 360.0, 1.0);
    let expected = normalize_to_scale((12.0 - longitude / 15.0) / 24.0, 1.0);
    if m0 - expected > 0.5 {
        m0 - 1.0
    } else if expected - m0 > 0.5 {
        m0 + 1.0
    } else {
        m0
    }
}

/// The universal time, in hours, at which the sun is highest.
fn corrected_transit(
    m0: f64,
    longitude: f64,
    sidereal_time: f64,
    right_ascension: f64,
    previous: f64,
    next: f64,
) -> f64 {
    let theta = unwind_angle(sidereal_time + 360.985_647 * m0);
    let a = unwind_angle(interpolate_angles(right_ascension, previous, next, m0));
    let h = quadrant_shift(theta + longitude - a);
    (m0 + h / -360.0) * 24.0
}

/// The universal time, in hours, at which the sun reaches a given altitude.
///
/// One correction pass is applied, which is what the reference does.
#[allow(clippy::too_many_arguments)]
fn corrected_hour_angle(
    m0: f64,
    altitude: f64,
    latitude: f64,
    longitude: f64,
    after_transit: bool,
    sidereal_time: f64,
    right_ascension: f64,
    previous_right_ascension: f64,
    next_right_ascension: f64,
    declination: f64,
    previous_declination: f64,
    next_declination: f64,
) -> f64 {
    let phi = to_radians(latitude);
    let h0 = to_radians(altitude);
    let d2 = to_radians(declination);
    let numerator = h0.sin() - phi.sin() * d2.sin();
    let denominator = phi.cos() * d2.cos();
    let h0_degrees = to_degrees((numerator / denominator).acos());
    let m = if after_transit {
        m0 + h0_degrees / 360.0
    } else {
        m0 - h0_degrees / 360.0
    };

    let theta = unwind_angle(sidereal_time + 360.985_647 * m);
    let a = unwind_angle(interpolate_angles(
        right_ascension,
        previous_right_ascension,
        next_right_ascension,
        m,
    ));
    let delta = interpolate(declination, previous_declination, next_declination, m);
    let h = theta + longitude - a;
    let altitude_now = altitude_of(latitude, delta, h);
    let term3 = altitude_now - altitude;
    let term4 = 360.0 * to_radians(delta).cos() * phi.cos() * to_radians(h).sin();
    (m + term3 / term4) * 24.0
}

/// Solar noon and the times of the sun's rise and set for one day.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SolarTime {
    /// The solar noon transit, in hours UTC.
    pub(crate) transit: f64,
    /// Sunrise, in hours UTC. Not a number when the sun never rises.
    pub(crate) sunrise: f64,
    /// Sunset, in hours UTC. Not a number when the sun never sets.
    pub(crate) sunset: f64,
    /// The first guess at the transit, kept for the hour-angle corrections.
    approx_transit: f64,
    solar: SolarCoordinates,
    previous: SolarCoordinates,
    next: SolarCoordinates,
    latitude: f64,
    longitude: f64,
}

/// The sun's altitude at sunrise and sunset, accounting for refraction and the
/// solar disc: 50 arc minutes below the horizon.
const SUNRISE_SUNSET_ALTITUDE: f64 = -50.0 / 60.0;

impl SolarTime {
    /// Computes the solar times for a date at a location.
    pub(crate) fn new(year: i32, month: u32, day: u32, latitude: f64, longitude: f64) -> Self {
        let julian_day = julian_day(year, month, day);
        let solar = SolarCoordinates::new(julian_day);
        let previous = SolarCoordinates::new(julian_day - 1.0);
        let next = SolarCoordinates::new(julian_day + 1.0);
        let m0 = approximate_transit(
            longitude,
            solar.apparent_sidereal_time,
            solar.right_ascension,
        );

        Self {
            transit: corrected_transit(
                m0,
                longitude,
                solar.apparent_sidereal_time,
                solar.right_ascension,
                previous.right_ascension,
                next.right_ascension,
            ),
            sunrise: corrected_hour_angle(
                m0,
                SUNRISE_SUNSET_ALTITUDE,
                latitude,
                longitude,
                false,
                solar.apparent_sidereal_time,
                solar.right_ascension,
                previous.right_ascension,
                next.right_ascension,
                solar.declination,
                previous.declination,
                next.declination,
            ),
            sunset: corrected_hour_angle(
                m0,
                SUNRISE_SUNSET_ALTITUDE,
                latitude,
                longitude,
                true,
                solar.apparent_sidereal_time,
                solar.right_ascension,
                previous.right_ascension,
                next.right_ascension,
                solar.declination,
                previous.declination,
                next.declination,
            ),
            approx_transit: m0,
            solar,
            previous,
            next,
            latitude,
            longitude,
        }
    }

    /// The time, in hours UTC, at which the sun reaches `altitude`.
    pub(crate) fn hour_angle(&self, altitude: f64, after_transit: bool) -> f64 {
        corrected_hour_angle(
            self.approx_transit,
            altitude,
            self.latitude,
            self.longitude,
            after_transit,
            self.solar.apparent_sidereal_time,
            self.solar.right_ascension,
            self.previous.right_ascension,
            self.next.right_ascension,
            self.solar.declination,
            self.previous.declination,
            self.next.declination,
        )
    }

    /// The time, in hours UTC, at which a shadow is `shadow_length` times the
    /// height of the object casting it.
    ///
    /// This is Asr: one shadow length for the majority, two for the Hanafi
    /// school. The shadow at noon is subtracted first, so the multiplier counts
    /// only the shadow the object adds.
    pub(crate) fn afternoon(&self, shadow_length: f64) -> f64 {
        let tangent = (self.latitude - self.solar.declination).abs();
        let inverse = shadow_length + to_radians(tangent).tan();
        let angle = to_degrees((1.0 / inverse).atan());
        self.hour_angle(angle, true)
    }
}

/// The morning twilight time the Moonsighting Committee uses at high latitude.
///
/// The angles are interpolated from four values that vary with latitude, along a
/// curve keyed to the solstice rather than the calendar.
pub(crate) fn season_adjusted_morning_twilight(
    latitude: f64,
    day_of_year: u32,
    year: i32,
    sunrise_seconds: f64,
) -> f64 {
    let lat = latitude.abs();
    let a = 75.0 + 28.65 / 55.0 * lat;
    let b = 75.0 + 19.44 / 55.0 * lat;
    let c = 75.0 + 32.74 / 55.0 * lat;
    let d = 75.0 + 48.1 / 55.0 * lat;
    let adjustment = twilight_adjustment(day_of_year, year, latitude, a, b, c, d);
    sunrise_seconds + (adjustment * -60.0).round()
}

/// The evening twilight time the Moonsighting Committee uses at high latitude.
///
/// Only the general shafaq is implemented, which is the default and what the
/// fixtures were produced with.
pub(crate) fn season_adjusted_evening_twilight(
    latitude: f64,
    day_of_year: u32,
    year: i32,
    sunset_seconds: f64,
) -> f64 {
    let lat = latitude.abs();
    let a = 75.0 + 25.6 / 55.0 * lat;
    let b = 75.0 + 2.05 / 55.0 * lat;
    let c = 75.0 - 9.21 / 55.0 * lat;
    let d = 75.0 + 6.14 / 55.0 * lat;
    let adjustment = twilight_adjustment(day_of_year, year, latitude, a, b, c, d);
    sunset_seconds + (adjustment * 60.0).round()
}

/// The piecewise curve the twilight adjustments follow, keyed to the solstice.
fn twilight_adjustment(
    day_of_year: u32,
    year: i32,
    latitude: f64,
    a: f64,
    b: f64,
    c: f64,
    d: f64,
) -> f64 {
    let dyy = days_since_solstice(day_of_year, year, latitude) as f64;
    if dyy < 91.0 {
        a + (b - a) / 91.0 * dyy
    } else if dyy < 137.0 {
        b + (c - b) / 46.0 * (dyy - 91.0)
    } else if dyy < 183.0 {
        c + (d - c) / 46.0 * (dyy - 137.0)
    } else if dyy < 229.0 {
        d + (c - d) / 46.0 * (dyy - 183.0)
    } else if dyy < 275.0 {
        c + (b - c) / 46.0 * (dyy - 229.0)
    } else {
        b + (a - b) / 91.0 * (dyy - 275.0)
    }
}

/// The day of the year, counting from one.
pub(crate) fn day_of_year(year: i32, month: u32, day: u32) -> u32 {
    const MONTH_LENGTHS: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let leap = is_leap_year(year);
    let mut total = day;
    for (index, length) in MONTH_LENGTHS.iter().enumerate() {
        if index as u32 + 1 < month {
            total += if index == 1 && leap { 29 } else { *length };
        }
    }
    total
}

/// Whether a year has 366 days.
pub(crate) fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Days since the summer solstice, wrapped to the year.
///
/// The twilight adjustments are written against this rather than the day of the
/// year, because the curve is symmetric about the solstice and the hemisphere
/// flips the reference point.
pub(crate) fn days_since_solstice(day_of_year: u32, year: i32, latitude: f64) -> i32 {
    let leap = is_leap_year(year);
    let days_in_year = if leap { 366 } else { 365 };
    let northern_offset = 10;
    let southern_offset = if leap { 173 } else { 172 };

    if latitude >= 0.0 {
        let mut days = day_of_year as i32 + northern_offset;
        if days >= days_in_year {
            days -= days_in_year;
        }
        days
    } else {
        let mut days = day_of_year as i32 - southern_offset;
        if days < 0 {
            days += days_in_year;
        }
        days
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_julian_day_matches_known_values() {
        // Meeus, example 7.a: 1957 October 4.81 is JD 2436116.31, so midnight
        // of that date is 2436115.5.
        assert!((julian_day(1957, 10, 4) - 2_436_115.5).abs() < 1e-6);
        // J2000.0.
        assert_eq!(julian_day(2000, 1, 1), 2_451_544.5);
    }

    #[test]
    fn the_sun_stands_still_at_the_solstices() {
        // The declination at the June solstice is about +23.44, at December
        // about -23.44. This pins the obliquity path without a fixture.
        let june = SolarCoordinates::new(julian_day(2026, 6, 21)).declination;
        let december = SolarCoordinates::new(julian_day(2026, 12, 21)).declination;
        assert!((june - 23.44).abs() < 0.05, "{june}");
        assert!((december + 23.44).abs() < 0.05, "{december}");
        // And the equinox is near zero.
        let march = SolarCoordinates::new(julian_day(2026, 3, 20)).declination;
        assert!(march.abs() < 0.5, "{march}");
    }

    #[test]
    fn leap_years_follow_the_gregorian_rule() {
        assert!(is_leap_year(2028));
        assert!(!is_leap_year(2026));
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2000));
    }

    #[test]
    fn the_day_of_year_counts_from_one() {
        assert_eq!(day_of_year(2026, 1, 1), 1);
        assert_eq!(day_of_year(2026, 12, 31), 365);
        assert_eq!(day_of_year(2028, 12, 31), 366);
        assert_eq!(day_of_year(2028, 3, 1), 61, "after a leap February");
        assert_eq!(day_of_year(2026, 3, 1), 60);
    }

    #[test]
    fn angles_are_reduced_to_their_range() {
        assert_eq!(unwind_angle(370.0), 10.0);
        assert_eq!(unwind_angle(-10.0), 350.0);
        assert_eq!(quadrant_shift(190.0), -170.0);
        assert_eq!(quadrant_shift(170.0), 170.0);
    }
}
