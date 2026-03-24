// Deliberately broken: invalid lifetime/generic syntax
pub trait BrokenTrait<'a, T: 'b {
    fn process(&self, input: T) -> &'a str;
}
