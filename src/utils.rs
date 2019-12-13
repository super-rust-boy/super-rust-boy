macro_rules! bit {
    ($bit_num:expr) => {
        (1 << $bit_num) as u8
    };
}

macro_rules! test_bit {
    ($val:expr, $bit_num:expr) => {
        ($val & bit!($bit_num)) != 0
    };
}