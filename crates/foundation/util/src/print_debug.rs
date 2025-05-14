/// For use during development. Instead of printing binary data as entirely binary,
/// stretches of ASCII alphanumeric characters (plus `.`, `-`, `_`) are printed as text,
/// with binary data interspersed.
///
/// For example:
/// `various_text-characters[0,1,2,3,]more_text[255,255,]`
#[expect(
    clippy::disallowed_macros,
    reason = "the prints used here should not end up in the final code of anything",
)]
pub fn print_debug(value: &[u8]) {
    let mut nums = value.iter().peekable();

    while nums.peek().is_some() {
        while let Some(&&num) = nums.peek() {
            if let Some(ch) = char::from_u32(u32::from(num)) {
                if ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '_' {
                    nums.next();
                    print!("{ch}");
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        print!("[");
        while let Some(&&num) = nums.peek() {
            if let Some(ch) = char::from_u32(u32::from(num)) {
                if ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '_' {
                    break;
                }
            }
            nums.next();
            print!("{num},");
        }
        print!("]");
    }
    println!();
}
