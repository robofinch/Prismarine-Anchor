#[macro_export]
macro_rules! declare_and_pub_use {
    () => {};
    ($($mod_name:ident);+ $(;)?) => {
        $(
            mod $mod_name;
            pub use $mod_name::*;
        )*
    };
}
