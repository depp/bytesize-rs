//! Human-readable byte sizes.

#[warn(missing_docs)]
use std::fmt;

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

#[cfg(test)]
mod test {
    use super::ByteSize;
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
}
