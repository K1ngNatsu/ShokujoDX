//! Named parameters of a sketch and dimensions with expressions (P14
//! Milestone 6, ADR 0078).
//!
//! A parameter has a name and an expression (`expr`), for example
//! `width = 80 mm` and `border = width / 20`. A dimension can have an
//! expression too (`Sketch::exprs`); after each change of a parameter, the
//! dimensions with expressions take their new values
//! (`Sketch::apply_parameters`), then the sketch is solved. A parameter
//! that uses itself, directly or through others, is refused (a loop).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::expr::{self, ExprError, Quantity};
use crate::{Constraint, ConstraintId, Sketch};

/// A named parameter of a sketch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub expr: String,
}

impl Sketch {
    pub fn parameter(&self, name: &str) -> Option<&Parameter> {
        self.parameters.iter().find(|p| p.name == name)
    }

    /// The values of all parameters, in the order of their dependencies.
    /// Fails for a parameter with an error, or for a loop.
    pub fn parameter_values(&self) -> Result<BTreeMap<String, Quantity>, ExprError> {
        values(&self.parameters, self.units)
    }

    /// The value of `text` with the parameters, as a length in mm (or, for
    /// `angle`, in degrees). The scale of the sketch (`expr_scale`) applies
    /// to lengths.
    pub fn eval(&self, text: &str, angle: bool) -> Result<f64, ExprError> {
        let values = self.parameter_values()?;
        self.eval_with(text, angle, &values)
    }

    fn eval_with(&self, text: &str, angle: bool, values: &BTreeMap<String, Quantity>) -> Result<f64, ExprError> {
        let q = expr::parse(text)?.eval(&|n| values.get(n).copied(), self.units)?;
        if angle {
            expr::to_angle(q)
        } else {
            Ok(expr::to_length(q, self.units)? * self.expr_scale())
        }
    }

    /// The factor of the lengths of expressions: a scale of the sketch
    /// (a scale of the design) after its parameters were set.
    pub fn expr_scale(&self) -> f64 {
        if self.expr_scale > 0.0 { self.expr_scale } else { 1.0 }
    }

    /// Adds parameter `name`, or changes its expression. The dimensions
    /// with expressions take their new values (solve after it). On an error
    /// the sketch does not change.
    pub fn set_parameter(&mut self, name: &str, text: &str) -> Result<(), ExprError> {
        let name = name.trim();
        if !expr::valid_name(name) {
            return Err(ExprError::BadName(name.to_string()));
        }
        let mut next = self.clone();
        match next.parameters.iter_mut().find(|p| p.name == name) {
            Some(p) => p.expr = text.trim().to_string(),
            None => next.parameters.push(Parameter {
                name: name.to_string(),
                expr: text.trim().to_string(),
            }),
        }
        next.apply_parameters()?;
        *self = next;
        Ok(())
    }

    /// Gives parameter `old` the name `new`, also in the expressions that
    /// use it.
    pub fn rename_parameter(&mut self, old: &str, new: &str) -> Result<(), ExprError> {
        let new = new.trim();
        if old == new {
            return Ok(());
        }
        if !expr::valid_name(new) {
            return Err(ExprError::BadName(new.to_string()));
        }
        if self.parameter(new).is_some() {
            return Err(ExprError::Taken(new.to_string()));
        }
        if self.parameter(old).is_none() {
            return Err(ExprError::Unknown(old.to_string()));
        }
        for p in &mut self.parameters {
            if p.name == old {
                p.name = new.to_string();
            }
            p.expr = expr::rename(&p.expr, old, new);
        }
        for e in self.exprs.values_mut() {
            *e = expr::rename(e, old, new);
        }
        Ok(())
    }

    /// Removes parameter `name` if no other value uses it.
    pub fn remove_parameter(&mut self, name: &str) -> Result<(), ExprError> {
        let uses = |text: &str| expr::parse(text).is_ok_and(|e| e.names().iter().any(|n| n == name));
        if self.parameters.iter().any(|p| p.name != name && uses(&p.expr)) || self.exprs.values().any(|e| uses(e)) {
            return Err(ExprError::InUse(name.to_string()));
        }
        self.parameters.retain(|p| p.name != name);
        Ok(())
    }

    /// Gives dimension `id` the expression `text` and its value now (solve
    /// after it). Returns the value.
    pub fn set_dimension_expr(&mut self, id: ConstraintId, text: &str) -> Result<f64, ExprError> {
        let c = *self.constraint(id).ok_or(ExprError::BadValue)?;
        if !c.is_dimension() {
            return Err(ExprError::BadValue);
        }
        if self.is_driven(id) {
            return Err(ExprError::Driven);
        }
        let v = self.eval(text, matches!(c, Constraint::Angle { .. }))?;
        self.set_value(id, v).map_err(|_| ExprError::BadValue)?;
        self.exprs.insert(id, text.trim().to_string());
        Ok(v)
    }

    /// The expression of dimension `id`, if it has one.
    pub fn dimension_expr(&self, id: ConstraintId) -> Option<&str> {
        self.exprs.get(&id).map(String::as_str)
    }

    /// Removes the expression of dimension `id` (its value stays).
    pub fn clear_dimension_expr(&mut self, id: ConstraintId) {
        self.exprs.remove(&id);
    }

    /// Gives each dimension with an expression its value from the
    /// parameters (solve after it). The expressions of removed dimensions
    /// go.
    pub fn apply_parameters(&mut self) -> Result<(), ExprError> {
        let values = self.parameter_values()?;
        let ids: Vec<ConstraintId> = self.constraints.iter().map(|c| c.id).collect();
        self.exprs.retain(|id, _| ids.contains(id));
        let exprs: Vec<(ConstraintId, String)> = self.exprs.iter().map(|(k, v)| (*k, v.clone())).collect();
        for (id, text) in exprs {
            let Some(c) = self.constraint(id).copied() else {
                continue;
            };
            let v = self.eval_with(&text, matches!(c, Constraint::Angle { .. }), &values)?;
            self.set_value(id, v).map_err(|_| ExprError::BadValue)?;
        }
        Ok(())
    }
}

/// The values of `params`: each parameter after the ones that it uses.
fn values(params: &[Parameter], units: expr::Units) -> Result<BTreeMap<String, Quantity>, ExprError> {
    let parsed: BTreeMap<&str, expr::Expr> = params
        .iter()
        .map(|p| Ok((p.name.as_str(), expr::parse(&p.expr)?)))
        .collect::<Result<_, ExprError>>()?;
    let mut done: BTreeMap<String, Quantity> = BTreeMap::new();
    // A walk into the names that each one uses; `path` finds the loops.
    fn visit<'a>(
        name: &'a str,
        parsed: &'a BTreeMap<&'a str, expr::Expr>,
        done: &mut BTreeMap<String, Quantity>,
        path: &mut Vec<&'a str>,
        units: expr::Units,
    ) -> Result<(), ExprError> {
        if done.contains_key(name) {
            return Ok(());
        }
        if let Some(i) = path.iter().position(|n| *n == name) {
            let mut lp: Vec<&str> = path[i..].to_vec();
            lp.push(name);
            return Err(ExprError::Loop(lp.join(" -> ")));
        }
        let Some(e) = parsed.get(name) else {
            return Err(ExprError::Unknown(name.to_string()));
        };
        path.push(name);
        for n in e.names() {
            let key = parsed.get_key_value(n.as_str()).map(|(k, _)| *k);
            match key {
                Some(k) => visit(k, parsed, done, path, units)?,
                None => return Err(ExprError::Unknown(n)),
            }
        }
        path.pop();
        let q = e.eval(&|n| done.get(n).copied(), units)?;
        done.insert(name.to_string(), q);
        Ok(())
    }
    for name in parsed.keys() {
        visit(name, &parsed, &mut done, &mut Vec::new(), units)?;
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    use emb_geom::Point;

    use super::*;
    use crate::expr::Units;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn parameters_drive_the_dimensions() {
        let mut s = Sketch::new();
        let l = s.add_line(p(0.0, 0.0), p(60.0, 0.0));
        let c = s.add_circle(p(30.0, 20.0), 3.0).unwrap();
        let len = s.add_constraint(Constraint::Length { line: l, mm: 60.0 }).unwrap();
        let r = s.add_constraint(Constraint::Radius { curve: c, mm: 3.0 }).unwrap();
        s.set_parameter("width", "80 mm").unwrap();
        s.set_parameter("border", "width / 20").unwrap();
        assert_eq!(s.set_dimension_expr(len, "width").unwrap(), 80.0);
        assert_eq!(s.set_dimension_expr(r, "border").unwrap(), 4.0);
        assert!(s.solve().converged);
        assert!((s.measure(&Constraint::Length { line: l, mm: 0.0 }).unwrap() - 80.0).abs() < 1e-6);
        // A new width: both dimensions follow.
        s.set_parameter("width", "100").unwrap();
        assert_eq!(s.constraint(len).unwrap().value(), Some(100.0));
        assert_eq!(s.constraint(r).unwrap().value(), Some(5.0));
        // A rename changes the expressions; a parameter in use stays.
        s.rename_parameter("width", "w").unwrap();
        assert_eq!(s.dimension_expr(len), Some("w"));
        assert_eq!(s.parameter("border").unwrap().expr, "w / 20");
        assert_eq!(s.remove_parameter("w"), Err(ExprError::InUse("w".into())));
        // Plain numbers in inches.
        s.units = Units::Inch;
        s.set_parameter("w", "2").unwrap();
        assert!((s.constraint(len).unwrap().value().unwrap() - 50.8).abs() < 1e-9);
        // Saved and loaded with the sketch.
        let back: Sketch = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn a_loop_of_parameters_is_refused() {
        let mut s = Sketch::new();
        s.set_parameter("a", "10 mm").unwrap();
        s.set_parameter("b", "a * 2").unwrap();
        s.set_parameter("c", "b + 1").unwrap();
        let before = s.clone();
        let e = s.set_parameter("a", "c / 2").unwrap_err();
        assert!(
            matches!(&e, ExprError::Loop(t) if t.contains('a') && t.contains('c')),
            "{e}"
        );
        assert_eq!(s, before, "no change");
        assert_eq!(s.set_parameter("a", "a + 1"), Err(ExprError::Loop("a -> a".into())));
        assert_eq!(s.set_parameter("d", "e"), Err(ExprError::Unknown("e".into())));
        assert_eq!(s.set_parameter("2x", "1"), Err(ExprError::BadName("2x".into())));
        assert_eq!(s.rename_parameter("a", "b"), Err(ExprError::Taken("b".into())));
    }

    #[test]
    fn a_wrong_value_for_a_dimension_is_refused() {
        let mut s = Sketch::new();
        let l = s.add_line(p(0.0, 0.0), p(10.0, 0.0));
        let len = s.add_constraint(Constraint::Length { line: l, mm: 10.0 }).unwrap();
        s.set_parameter("w", "10 mm").unwrap();
        assert_eq!(s.set_dimension_expr(len, "w * w"), Err(ExprError::NotLength));
        assert_eq!(s.set_dimension_expr(len, "w - 20"), Err(ExprError::BadValue));
        s.set_dimension_expr(len, "w").unwrap();
        // A parameter change that makes a dimension wrong is refused.
        assert_eq!(s.set_parameter("w", "-5 mm"), Err(ExprError::BadValue));
        assert_eq!(s.constraint(len).unwrap().value(), Some(10.0));
        // The scale of a design scales the lengths of expressions.
        s.expr_scale = 2.0;
        s.apply_parameters().unwrap();
        assert_eq!(s.constraint(len).unwrap().value(), Some(20.0));
    }
}
