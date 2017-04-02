// Common functions and things

macro_rules! makemap {
    ($($key: expr => $val: expr),*) => {
        {
            let mut map = HashMap::new();
            $(map.insert($key, $val);)*
            map
        }
    };
}
