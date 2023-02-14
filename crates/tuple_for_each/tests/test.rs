use tuple_for_each::tuple_for_each;

trait Base {
    fn foo(&self) -> u32;
    fn bar(&self);
}

struct A;
struct B;

impl Base for A {
    fn foo(&self) -> u32 {
        1
    }
    fn bar(&self) {
        println!("I'am A")
    }
}

impl Base for B {
    fn foo(&self) -> u32 {
        2
    }
    fn bar(&self) {
        println!("I'am B")
    }
}

#[tuple_for_each(Base)]
struct Tuple(A, B);

#[test]
fn test_tuple_for_each() {
    let t = Tuple(A, B);
    assert_eq!(t.len(), 2);
    t.for_each(|x| {
        print!("{}: ", x.foo());
        x.bar();
    });
}
