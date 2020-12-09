#[macro_use]
extern crate lazy_static;

mod types;

use types::name::Name;


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
