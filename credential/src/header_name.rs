use once_cell::sync::Lazy;
use std::{borrow::Cow, collections::HashSet};

pub(super) fn make_header_name(header_name: Cow<str>) -> Cow<str> {
    let mut need_not_clone = header_name
        .chars()
        .any(|header_char| !HEADER_NAME_TOKEN.contains(&header_char));
    if need_not_clone {
        let mut upper = true;
        need_not_clone = header_name.chars().all(|header_char| {
            if (upper && header_char.is_lowercase()) || (!upper && header_char.is_uppercase()) {
                false
            } else {
                upper = header_char == '-';
                true
            }
        })
    };
    if need_not_clone {
        return header_name;
    }

    let mut upper = true;
    let mut new_header_name = String::with_capacity(header_name.len());
    for header_char in header_name.chars() {
        if upper && header_char.is_lowercase() {
            new_header_name.push(header_char.to_ascii_uppercase());
        } else if !upper && header_char.is_uppercase() {
            new_header_name.push(header_char.to_ascii_lowercase());
        } else {
            new_header_name.push(header_char);
        }
        upper = header_char == '-';
    }
    new_header_name.into()
}

static HEADER_NAME_TOKEN: Lazy<HashSet<char>> = Lazy::new(|| {
    let mut set = HashSet::with_capacity(127);
    set.insert('!');
    set.insert('#');
    set.insert('$');
    set.insert('%');
    set.insert('&');
    set.insert('\'');
    set.insert('*');
    set.insert('+');
    set.insert('-');
    set.insert('.');
    set.insert('0');
    set.insert('1');
    set.insert('2');
    set.insert('3');
    set.insert('4');
    set.insert('5');
    set.insert('6');
    set.insert('7');
    set.insert('8');
    set.insert('9');
    set.insert('A');
    set.insert('B');
    set.insert('C');
    set.insert('D');
    set.insert('E');
    set.insert('F');
    set.insert('G');
    set.insert('H');
    set.insert('I');
    set.insert('J');
    set.insert('K');
    set.insert('L');
    set.insert('M');
    set.insert('N');
    set.insert('O');
    set.insert('P');
    set.insert('Q');
    set.insert('R');
    set.insert('S');
    set.insert('T');
    set.insert('U');
    set.insert('W');
    set.insert('V');
    set.insert('X');
    set.insert('Y');
    set.insert('Z');
    set.insert('^');
    set.insert('_');
    set.insert('`');
    set.insert('a');
    set.insert('b');
    set.insert('c');
    set.insert('d');
    set.insert('e');
    set.insert('f');
    set.insert('g');
    set.insert('h');
    set.insert('i');
    set.insert('j');
    set.insert('k');
    set.insert('l');
    set.insert('m');
    set.insert('n');
    set.insert('o');
    set.insert('p');
    set.insert('q');
    set.insert('r');
    set.insert('s');
    set.insert('t');
    set.insert('u');
    set.insert('v');
    set.insert('w');
    set.insert('x');
    set.insert('y');
    set.insert('z');
    set.insert('|');
    set.insert('~');
    set
});
