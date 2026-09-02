pub const STARTING_BUF_CAPACITY: usize = 16;

pub fn starting_buf(ch: char) -> String {
  let mut buf = String::with_capacity(STARTING_BUF_CAPACITY);
  buf.push(ch);
  buf
}
