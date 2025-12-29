pub const KIBIBYTE: usize = 1024;
pub const MEBIBYTE: usize = KIBIBYTE * KIBIBYTE;

#[macro_export]
macro_rules! return_if_none {
    ($try:expr, $message:expr) => {{
        let res = $try;
        if res.is_none() {
            return Err($message);
        }
        res.unwrap()
    }};
}
#[macro_export]
macro_rules! gen_error_cascade {
    ($from:ty, $to:ty, $entry:ident) => {
        impl core::convert::From<$from> for $to {
            fn from(e: $from) -> $to {
                <$to>::$entry(e)
            }
        }
    };
}

#[macro_export]
macro_rules! unwrap_or_map_err {
    ($try:expr, $from_error:ty) => {
        $try.map_err(|err| <$from_error>::from(err))?
    };
}
#[macro_export]
macro_rules! equals_chain {
    ($check:expr; $($against:expr),+) => ((  $( $check == $against || )+ false ));
}

#[macro_export]
macro_rules! pseudo_const {
    ($name:ident: $typ:ty = $val:expr) => {
        #[inline(always)]
        fn $name() -> $typ {
            $val
        }
    };
}

#[macro_export]
macro_rules! pub_pseudo_const {
    ($name:ident: $typ:ty = $val:expr) => {
        #[inline(always)]
        pub fn $name() -> $typ {
            $val
        }
    };
}
