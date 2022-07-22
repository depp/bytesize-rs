# ByteSize (Rust)

**Simple** and **correct.**

ByteSize is a Rust library for formatting and parsing numbers as byte sizes for humans. Numbers are formatted with three digits of precision, using SI prefixes, and the unit “B” for bytes. For example, 1000 is formatted as “1.00 KB”. The numbers are rounded using round-to-even, which is the familiar method used by `std::fmt`.

## Formatting

The precision cannot be changed. Non-decimal prefixes are not produced: no kibibytes, no powers of two. Wrap the number in the `ByteSize` struct and print it with `std::fmt`.

```rust
/// A byte size which is displayed using SI prefixes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteSize(pub u64);
```

All corner cases should be handled correctly and you should never see unusual or unexpected output. You should always see exactly three digits, except for inputs under 100.

Some test cases:

```
0 => "0 B"
999 => "999 B"
1000 => "1.00 kB"
1005 => "1.00 kB"
1006 => "1.01 kB"
1014 => "1.01 kB"
1015 => "1.02 kB"
9995 => "10.0 kB"
314000 => "314 kB"
18400000000000000000 => "18.4 EB"
```

## Parsing

The parser understands decimal numbers, decimal SI prefixes, and binary SI prefixes. It is not case-sensitive. A sequence ASCII space (0x20) and tab (0x09) characters may optionally appear between the number and the units. The parser will recognize all positve prefixes which are powers of 1,000, including prefixes that will overflow (yotta and zetta).

Some test cases:

```
"0" => 0
"1" => 1
"555k" => 555000
"15 EiB" => 17293822569102704640
"1.5 mb" => 1500000
"2gi" => 2147483648
"0.001 zb" => 1000000000000000000
```

## License

This library is licensed under the terms of both the MIT license and the Apache 2.0 license. See [LICENSE-MIT.txt](LICENSE-MIT.txt) and [LICENSE-APACHE.txt](LICENSE-APACHE.txt) for details.
