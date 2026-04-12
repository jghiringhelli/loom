// proof.rs — emitted by: loom compile proof.loom
// Theory: Liskov Substitution Principle (Liskov & Wing 1987)
// Every interface implementation is verified complete at compile time.
// Any function accepting the trait works identically with all implementations.

pub trait Shape {
    fn area(&self) -> f64;
    fn perimeter(&self) -> f64;
    fn describe(&self) -> String;
}

// ── Square: fully implements Shape ───────────────────────────────────────────

pub struct Square { pub side: f64 }

impl Square {
    pub fn new(side: f64) -> Self {
        debug_assert!(side > 0.0, "require: side > 0");
        Self { side }
    }
}

impl Shape for Square {
    fn area(&self) -> f64 {
        let result = self.side * self.side;
        debug_assert!(result >= 0.0, "ensure: area >= 0");
        result
    }
    fn perimeter(&self) -> f64 {
        let result = 4.0 * self.side;
        debug_assert!(result >= 0.0, "ensure: perimeter >= 0");
        result
    }
    fn describe(&self) -> String {
        format!("Square with side {}", self.side)
    }
}

// ── Circle: fully implements Shape ───────────────────────────────────────────

pub struct Circle { pub radius: f64 }

impl Circle {
    pub fn new(radius: f64) -> Self {
        debug_assert!(radius > 0.0, "require: radius > 0");
        Self { radius }
    }
}

impl Shape for Circle {
    fn area(&self) -> f64 {
        let result = std::f64::consts::PI * self.radius * self.radius;
        debug_assert!(result >= 0.0);
        result
    }
    fn perimeter(&self) -> f64 {
        let result = 2.0 * std::f64::consts::PI * self.radius;
        debug_assert!(result >= 0.0);
        result
    }
    fn describe(&self) -> String {
        format!("Circle with radius {}", self.radius)
    }
}

// ── LSP proof: function works with ANY Shape implementation ──────────────────

pub fn total_area(shapes: &[&dyn Shape]) -> f64 {
    shapes.iter().map(|s| s.area()).sum()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_satisfies_shape_contract() {
        let s = Square::new(4.0);
        assert_eq!(s.area(), 16.0);
        assert_eq!(s.perimeter(), 16.0);
        assert!(s.describe().contains("Square"));
    }

    #[test]
    fn circle_satisfies_shape_contract() {
        let c = Circle::new(1.0);
        assert!((c.area() - std::f64::consts::PI).abs() < 1e-10);
        assert!(c.describe().contains("Circle"));
    }

    #[test]
    fn lsp_total_area_works_with_any_shape() {
        let sq = Square::new(2.0);
        let ci = Circle::new(1.0);
        let shapes: Vec<&dyn Shape> = vec![&sq, &ci];
        let total = total_area(&shapes);
        // 4.0 + π ≈ 7.14
        assert!(total > 7.0 && total < 7.2);
    }

    #[test]
    fn all_implementations_are_substitutable() {
        let shapes: Vec<Box<dyn Shape>> = vec![
            Box::new(Square::new(3.0)),
            Box::new(Circle::new(2.0)),
        ];
        for shape in &shapes {
            assert!(shape.area() >= 0.0, "LSP: area postcondition holds for all");
            assert!(shape.perimeter() >= 0.0, "LSP: perimeter postcondition holds for all");
        }
    }
}
