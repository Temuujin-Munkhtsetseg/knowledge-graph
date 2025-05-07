use parser_core::add;

pub fn add_two_numbers(a: i32, b: i32) -> i32 {
    add(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add_two_numbers(2, 2);
        assert_eq!(result, 4);
    }
}
