//! A gallery of shapes behind one enum: `cargo run --example shapes`.

#[whichever::dispatchable]
pub trait Shape {
    fn area(&self) -> f64;
    fn name(&self) -> &'static str {
        "shape"
    }
    /// Two different kinds of shape are never congruent.
    #[dispatch(mismatch = false)]
    fn congruent(&self, other: &Self) -> bool;
    fn scaled(&self, factor: f64) -> Self;
}

pub struct Circle(pub f64);
pub struct Rect(pub f64, pub f64);

impl Shape for Circle {
    fn area(&self) -> f64 {
        std::f64::consts::PI * self.0 * self.0
    }
    fn name(&self) -> &'static str {
        "circle"
    }
    fn congruent(&self, other: &Self) -> bool {
        self.0 == other.0
    }
    fn scaled(&self, factor: f64) -> Self {
        Circle(self.0 * factor)
    }
}

impl Shape for Rect {
    fn area(&self) -> f64 {
        self.0 * self.1
    }
    fn congruent(&self, other: &Self) -> bool {
        (self.0, self.1) == (other.0, other.1)
    }
    fn scaled(&self, factor: f64) -> Self {
        Rect(self.0 * factor, self.1 * factor)
    }
}

#[whichever::dispatch(Shape)]
pub enum AnyShape {
    Circle(Circle),
    Rect(Rect),
}

fn main() {
    let gallery: Vec<AnyShape> = vec![Circle(1.0).into(), Rect(2.0, 3.0).into()];
    for shape in &gallery {
        println!("{:>6}: area {:.2}", shape.name(), shape.area());
    }
    let doubled = gallery[1].scaled(2.0);
    println!("doubled rect: area {:.2}", doubled.area());
    println!(
        "circle congruent with itself: {}",
        gallery[0].congruent(&Circle(1.0).into())
    );
    println!(
        "circle congruent with rect: {}",
        gallery[0].congruent(&gallery[1])
    );
}
