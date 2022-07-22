//! Human-readable byte sizes.

#[warn(missing_docs)]
use std::error;
use std::fmt;
use std::str;
use std::str::FromStr;

/// A byte size which is displayed using SI prefixes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteSize(pub u64);

impl fmt::Display for ByteSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const PREFIXES: &'static [u8] = b"kMGTPE";
        let ByteSize(size) = *self;
        if size < 1000 {
            return write!(f, "{} B", size);
        }

        // Divide it down, trying each next prefix, until the units is under
        // 1000. The original number will be close to
        //
        // (units + millis/1000) * 1000^prefix.
        let mut prefix = 0;
        let mut units = size;
        let mut millis;
        let mut is_exact = true;
        loop {
            millis = units % 1000;
            units /= 1000;
            if units < 1000 {
                break;
            }
            if millis > 0 {
                is_exact = false;
            }
            prefix += 1;
        }
        let mut units = units as u32;
        let millis = millis as u32;

        if units < 10 {
            // 1 digit before the decimal point.
            let mut frac = millis / 10;
            let rem = millis % 10;
            if rem > 5 || (rem == 5 && ((frac & 1) != 0 || !is_exact)) {
                frac += 1;
                if frac == 100 {
                    frac = 0;
                    units += 1;
                    if units == 10 {
                        return write!(f, "10.0 {}B", PREFIXES[prefix] as char);
                    }
                }
            }
            write!(f, "{}.{:02} {}B", units, frac, PREFIXES[prefix] as char)
        } else if units < 100 {
            // 2 digits before the decimal point.
            let mut frac = millis / 100;
            let rem = millis % 100;
            if rem > 50 || (rem == 50 && ((frac & 1) != 0 || !is_exact)) {
                frac += 1;
                if frac == 10 {
                    frac = 0;
                    units += 1;
                    if units == 100 {
                        return write!(f, "100 {}B", PREFIXES[prefix] as char);
                    }
                }
            }
            write!(f, "{}.{} {}B", units, frac, PREFIXES[prefix] as char)
        } else {
            // 3 digits before the decimal point.
            if millis > 500 || (millis == 500 && ((units & 1) != 0 || !is_exact)) {
                units += 1;
            }
            if units >= 1000 {
                write!(f, "1.00 {}B", PREFIXES[prefix + 1] as char)
            } else {
                write!(f, "{} {}B", units, PREFIXES[prefix] as char)
            }
        }
    }
}

fn parse_prefix(pfx: u8) -> Option<u32> {
    Some(match pfx & !0x20 {
        b'K' => 1,
        b'M' => 2,
        b'G' => 3,
        b'T' => 4,
        b'P' => 5,
        b'E' => 6,
        b'Z' => 7,
        b'Y' => 8,
        _ => return None,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseByteSizeError {
    Empty,
    InvalidNumber,
    InvalidUnits,
    Overflow,
}

impl fmt::Display for ParseByteSizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ParseByteSizeError::*;
        f.write_str(match *self {
            Empty => "cannot parse empty string",
            InvalidNumber => "string does not start with invalid number",
            InvalidUnits => "string has invalid units",
            Overflow => "number is too large",
        })
    }
}

impl error::Error for ParseByteSizeError {}

impl FromStr for ByteSize {
    type Err = ParseByteSizeError;
    fn from_str(src: &str) -> Result<Self, ParseByteSizeError> {
        use ParseByteSizeError as PE;

        // Split the number into number and units, and find the decimal point.
        let src = src.as_bytes();
        let mut point: Option<usize> = None;
        let mut num_end = src.len();
        for (n, &c) in src.iter().enumerate() {
            match c {
                b'0'..=b'9' => {}
                b'.' => {
                    if point.is_some() {
                        return Err(PE::InvalidNumber);
                    } else {
                        point = Some(n);
                    }
                }
                _ => {
                    num_end = n;
                    break;
                }
            }
        }
        let (num, units) = src.split_at(num_end);
        let units = match units.iter().position(|&c| c != b' ' && c != b'\t') {
            None => units,
            Some(n) => &units[n..],
        };

        // Parse the units.
        let units = match units.split_last() {
            Some((&c, rest)) if c == b'b' || c == b'B' => rest,
            _ => units,
        };
        let mut binary = false;
        let mut scale: u32 = 0;
        if let Some((&c, rest)) = units.split_first() {
            if rest == b"i" || rest == b"I" {
                binary = true;
            } else if !rest.is_empty() {
                return Err(PE::InvalidUnits);
            }
            scale = match parse_prefix(c) {
                None => return Err(PE::InvalidUnits),
                Some(x) => x,
            };
        }

        Ok(ByteSize(if binary {
            // Parse a number with a binary prefix. We just farm this out to
            // f64, since we can adjust the exponent using the prefix. This
            // limits us to 53 bits of precision and may result in double
            // rounding... oh well, another problem with binary prefixes.
            //
            // Note: Safe because this is sliced from a &str to begin with.
            let f = match f64::from_str(unsafe { str::from_utf8_unchecked(num) }) {
                Ok(f) => f,
                Err(_) => return Err(PE::InvalidNumber),
            };
            let f = (f * 1024.0f64.powi(scale as i32)).round();
            if f >= 2.0f64.powi(64) {
                return Err(PE::Overflow);
            }
            f as u64
        } else {
            // Parse with decimal prefix.

            // Calculate how many digits before decimal point, after adjusting
            // for the SI prefix.
            let mut idigits = match point {
                Some(x) => x,
                None => num.len(),
            } + 3 * (scale as usize);
            let mut v: u64 = 0;
            let mut frac: &[u8] = &[];

            // Parse integer digits.
            for (n, &c) in num.iter().enumerate() {
                if c == b'.' {
                    continue;
                }
                if idigits == 0 {
                    frac = &num[n..];
                    break;
                }
                idigits -= 1;
                v = match v.checked_mul(10) {
                    Some(v) => v,
                    None => return Err(PE::Overflow),
                };
                v = match v.checked_add((c as u64) & 15) {
                    Some(v) => v,
                    None => return Err(PE::Overflow),
                };
            }
            for _ in 0..idigits {
                v = match v.checked_mul(10) {
                    Some(v) => v,
                    None => return Err(PE::Overflow),
                };
            }

            // Round to even.
            if let Some((&c, rest)) = frac.split_first() {
                let round_up = if c > b'5' {
                    true
                } else if c < b'5' {
                    false
                } else if v & 1 != 0 {
                    true
                } else {
                    rest.iter().any(|&c| c != b'0')
                };
                if round_up {
                    v = match v.checked_add(1) {
                        Some(v) => v,
                        None => return Err(PE::Overflow),
                    };
                }
            }
            v
        }))
    }
}

#[cfg(test)]
mod test {
    use super::ByteSize;
    use std::str::FromStr;
    use std::string::ToString;

    #[test]
    fn test_bytesize() {
        const TESTS: &'static [(u64, &'static str)] = &[
            (0, "0 B"),
            (5, "5 B"),
            (20, "20 B"),
            (100, "100 B"),
            (500, "500 B"),
            (999, "999 B"),
            (1000, "1.00 kB"),
            (1005, "1.00 kB"),
            (1006, "1.01 kB"),
            (2334, "2.33 kB"),
            (2335, "2.34 kB"),
            (2995, "3.00 kB"),
            (9994, "9.99 kB"),
            (9995, "10.0 kB"),
            (10000, "10.0 kB"),
            (10050, "10.0 kB"),
            (10061, "10.1 kB"),
            (99949, "99.9 kB"),
            (99950, "100 kB"),
            (999499, "999 kB"),
            (999500, "1.00 MB"),
            (1000000, "1.00 MB"),
            (952500000, "952 MB"),
            (952500001, "953 MB"),
            (1000000000, "1.00 GB"),
            (2300000000000, "2.30 TB"),
            (9700000000000000, "9.70 PB"),
            (u64::max_value(), "18.4 EB"),
        ];
        let mut success = true;
        for (num, expect) in TESTS.iter() {
            let num = *num;
            let expect = *expect;
            let out = ByteSize(num).to_string();
            if out.as_str() != expect {
                success = false;
                eprintln!("{:?} => {:?}, expect {:?}", num, out, expect);
            }
        }
        assert!(success);
    }

    #[test]
    fn test_parse() {
        // SI sizes.
        const SIZEK: u64 = 1000;
        const SIZEM: u64 = 1000 * SIZEK;
        const SIZEG: u64 = 1000 * SIZEM;
        const SIZET: u64 = 1000 * SIZEG;
        const SIZEP: u64 = 1000 * SIZET;
        const SIZEE: u64 = 1000 * SIZEP;

        // Binary prefix sizes.
        const SIZEKI: u64 = 1 << 10;
        const SIZEMI: u64 = 1 << 20;
        const SIZEGI: u64 = 1 << 30;
        const SIZETI: u64 = 1 << 40;
        const SIZEPI: u64 = 1 << 50;
        const SIZEEI: u64 = 1 << 60;

        const TESTS: &'static [(&'static str, &'static str, u64)] = &[
            // Integers + decimal units.
            ("0", "", 0),
            ("23", "", 23),
            ("103", "k", 103 * SIZEK),
            ("12", "m", 12 * SIZEM),
            ("3", "g", 3 * SIZEG),
            ("715", "t", 715 * SIZET),
            ("9", "p", 9 * SIZEP),
            ("5", "e", 5 * SIZEE),
            ("18446744073709551615", "", u64::MAX),
            // Fractions + decimal units.
            ("1.205", "k", 1205),
            ("16.6", "m", 16600 * SIZEK),
            ("18.446744073709551615", "e", u64::MAX),
            ("0.018", "z", 18 * SIZEE),
            // Fractions of a byte round to even.
            ("1.4", "", 1),
            ("1.5", "", 2),
            ("1.9", "", 2),
            ("2.1", "", 2),
            ("2.5000", "", 2),
            ("2.50001", "", 3),
            ("1.2306", "k", 1231),
            // Integers + binary units.
            ("103", "ki", 103 * SIZEKI),
            ("99", "mi", 99 * SIZEMI),
            ("2", "gi", 2 * SIZEGI),
            ("999", "ti", 999 * SIZETI),
            ("8", "pi", 8 * SIZEPI),
            ("15", "ei", 15 * SIZEEI),
            // Fractions + binary units.
            ("103.5", "ki", 103 * SIZEKI + SIZEKI / 2),
            ("593.2", "mi", 622015283),
            ("12.25", "pi", 12 * SIZEPI + SIZEPI / 4),
        ];

        let mut success = true;
        let mut s = String::new();
        for &(num, unit, expect) in TESTS.iter() {
            for i in 0..4 {
                s.clear();
                s.push_str(num);
                match i {
                    0 => s.push_str(unit),
                    1 => {
                        s.push_str(unit);
                        s.push('b');
                    }
                    2 => {
                        s.push_str(unit);
                        s.make_ascii_uppercase();
                        s.push('B');
                    }
                    3 => {
                        if !unit.is_empty() {
                            s.push(' ');
                            s.push_str(unit);
                        }
                    }
                    _ => panic!(),
                }
                match ByteSize::from_str(s.as_str()) {
                    Ok(ByteSize(x)) => {
                        if x != expect {
                            eprintln!("Parse({:?}) = {}, expect {}", &s, x, expect);
                            success = false;
                        }
                    }
                    Err(e) => {
                        eprintln!("Parse({:?}): {}", &s, e);
                        success = false;
                    }
                }
            }
        }
        assert!(success);
    }
}
