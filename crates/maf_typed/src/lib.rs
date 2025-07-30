use facet::Facet;

#[derive(Facet)]
pub struct Person<T> {
    id: u64,
    name: String,
    age: u8,
    data: T,
}

fn a(person: Person<String>) {}

pub fn test() {
    println!("{:?}", Person::<String>::SHAPE);
}
