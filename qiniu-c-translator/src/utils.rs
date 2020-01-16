use std::iter;

pub fn generate_prefix_spaces(level: usize, spaces: usize) -> String {
    iter::repeat(' ').take(level * spaces).collect()
}
