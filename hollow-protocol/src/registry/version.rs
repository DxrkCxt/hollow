// SPDX-License-Identifier: GPL-3.0-only

macro_rules! versions {
    ($(($variant:ident, $num:expr, $name:expr)),* $(,)?) => {
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        pub enum Version {
            $($variant),*
        }

        impl Version {
            /// All versions in declaration order (UNDEFINED first), equivalent to `Version.values()`.
            pub const VALUES: &'static [Version] = &[ $(Version::$variant),* ];

            pub fn protocol_number(self) -> i32 {
                match self { $(Version::$variant => $num),* }
            }

            pub fn display_name(self) -> &'static str {
                match self { $(Version::$variant => $name),* }
            }
        }
    };
}

versions! {
    (Undefined, -1, "UNDEFINED"),
    (V1_7_2, 4, "1.7.2"),
    (V1_7_6, 5, "1.7.6"),
    (V1_8, 47, "1.8"),
    (V1_9, 107, "1.9"),
    (V1_9_1, 108, "1.9.1"),
    (V1_9_2, 109, "1.9.2"),
    (V1_9_4, 110, "1.9.4"),
    (V1_10, 210, "1.10"),
    (V1_11, 315, "1.11"),
    (V1_11_1, 316, "1.11.1"),
    (V1_12, 335, "1.12"),
    (V1_12_1, 338, "1.12.1"),
    (V1_12_2, 340, "1.12.2"),
    (V1_13, 393, "1.13"),
    (V1_13_1, 401, "1.13.1"),
    (V1_13_2, 404, "1.13.2"),
    (V1_14, 477, "1.14"),
    (V1_14_1, 480, "1.14.1"),
    (V1_14_2, 485, "1.14.2"),
    (V1_14_3, 490, "1.14.3"),
    (V1_14_4, 498, "1.14.4"),
    (V1_15, 573, "1.15"),
    (V1_15_1, 575, "1.15.1"),
    (V1_15_2, 578, "1.15.2"),
    (V1_16, 735, "1.16"),
    (V1_16_1, 736, "1.16.1"),
    (V1_16_2, 751, "1.16.2"),
    (V1_16_3, 753, "1.16.3"),
    (V1_16_4, 754, "1.16.4"),
    (V1_17, 755, "1.17"),
    (V1_17_1, 756, "1.17.1"),
    (V1_18, 757, "1.18"),
    (V1_18_2, 758, "1.18.2"),
    (V1_19, 759, "1.19"),
    (V1_19_1, 760, "1.19.1"),
    (V1_19_3, 761, "1.19.3"),
    (V1_19_4, 762, "1.19.4"),
    (V1_20, 763, "1.20"),
    (V1_20_2, 764, "1.20.2"),
    (V1_20_3, 765, "1.20.3"),
    (V1_20_5, 766, "1.20.5"),
    (V1_21, 767, "1.21"),
    (V1_21_2, 768, "1.21.2"),
    (V1_21_4, 769, "1.21.4"),
    (V1_21_5, 770, "1.21.5"),
    (V1_21_6, 771, "1.21.6"),
    (V1_21_7, 772, "1.21.7"),
    (V1_21_9, 773, "1.21.9"),
    (V1_21_11, 774, "1.21.11"),
    (V26_1, 775, "26.1"),
}

impl Version {
    fn index(self) -> usize {
        Self::VALUES
            .iter()
            .position(|&v| v == self)
            .expect("version present in VALUES")
    }

    /// Previous version in declaration order, equivalent to the `prev` field.
    pub fn prev(self) -> Option<Version> {
        let i = self.index();
        if i == 0 { None } else { Some(Self::VALUES[i - 1]) }
    }

    pub fn more(self, other: Version) -> bool {
        self.protocol_number() > other.protocol_number()
    }

    pub fn more_or_equal(self, other: Version) -> bool {
        self.protocol_number() >= other.protocol_number()
    }

    pub fn less(self, other: Version) -> bool {
        self.protocol_number() < other.protocol_number()
    }

    pub fn less_or_equal(self, other: Version) -> bool {
        self.protocol_number() <= other.protocol_number()
    }

    pub fn from_to(self, min: Version, max: Version) -> bool {
        let p = self.protocol_number();
        p >= min.protocol_number() && p <= max.protocol_number()
    }

    pub fn is_supported(self) -> bool {
        self != Version::Undefined
    }

    /// Minimum supported version, equivalent to `Version.getMin()`.
    pub fn min() -> Version {
        Version::V1_7_2
    }

    /// Maximum supported version, equivalent to `Version.getMax()`.
    pub fn max() -> Version {
        *Self::VALUES.last().expect("VALUES non-empty")
    }

    /// Resolve a version from a protocol number, equivalent to `Version.of(int)`.
    pub fn of(protocol_number: i32) -> Version {
        Self::VALUES
            .iter()
            .copied()
            .find(|v| v.protocol_number() == protocol_number)
            .unwrap_or(Version::Undefined)
    }
}

#[cfg(test)]
mod tests {
    use super::Version;

    #[test]
    fn min_max() {
        assert_eq!(Version::min(), Version::V1_7_2);
        assert_eq!(Version::max(), Version::V26_1);
    }

    #[test]
    fn of_known_and_unknown() {
        assert_eq!(Version::of(47), Version::V1_8);
        assert_eq!(Version::of(767), Version::V1_21);
        assert_eq!(Version::of(-1), Version::Undefined);
        assert_eq!(Version::of(99999), Version::Undefined);
    }

    #[test]
    fn prev_chain() {
        assert_eq!(Version::V1_8.prev(), Some(Version::V1_7_6));
        assert_eq!(Version::V1_7_2.prev(), Some(Version::Undefined));
        assert_eq!(Version::Undefined.prev(), None);
    }

    #[test]
    fn comparisons() {
        assert!(Version::V1_21.more(Version::V1_8));
        assert!(Version::V1_8.less(Version::V1_21));
        assert!(Version::V1_16.more_or_equal(Version::V1_16));
        assert!(Version::V1_18_2.from_to(Version::V1_17, Version::V1_20));
        assert!(!Version::V1_21.from_to(Version::V1_17, Version::V1_20));
    }

    #[test]
    fn supported() {
        assert!(!Version::Undefined.is_supported());
        assert!(Version::V1_8.is_supported());
    }
}
