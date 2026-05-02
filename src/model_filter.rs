use glob_match::glob_match;
use serde_json::Value;

pub struct ModelFilter {
    include: Vec<String>,
    exclude: Vec<String>,
}

impl ModelFilter {
    pub fn new(include: Vec<String>, exclude: Vec<String>) -> Self {
        Self { include, exclude }
    }

    pub fn matches(&self, model_name: &str) -> bool {
        let included = self.include.is_empty()
            || self.include.iter().all(|p| p == "*")
            || self.include.iter().any(|p| glob_match(p, model_name));

        if !included {
            return false;
        }

        let excluded = self.exclude.iter().any(|p| glob_match(p, model_name));

        !excluded
    }

    pub fn filter(&self, models: Vec<Value>) -> Vec<Value> {
        models
            .into_iter()
            .filter(|m| {
                m.get("model")
                    .and_then(Value::as_str)
                    .map(|name| self.matches(name))
                    .unwrap_or(false)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn empty_include_matches_all() {
        let f = ModelFilter::new(vec![], vec![]);
        assert!(f.matches("res.partner"));
        assert!(f.matches("sale.order"));
    }

    #[test]
    fn wildcard_include_matches_all() {
        let f = ModelFilter::new(vec!["*".into()], vec![]);
        assert!(f.matches("res.partner"));
    }

    #[test]
    fn specific_include_pattern() {
        let f = ModelFilter::new(vec!["res.*".into()], vec![]);
        assert!(f.matches("res.partner"));
        assert!(!f.matches("sale.order"));
    }

    #[test]
    fn multiple_include_patterns() {
        let f = ModelFilter::new(vec!["res.*".into(), "sale.*".into()], vec![]);
        assert!(f.matches("res.partner"));
        assert!(f.matches("sale.order"));
        assert!(!f.matches("stock.move"));
    }

    #[test]
    fn exclude_takes_precedence() {
        let f = ModelFilter::new(vec!["res.*".into()], vec!["res.config.*".into()]);
        assert!(f.matches("res.partner"));
        assert!(!f.matches("res.config.settings"));
    }

    #[test]
    fn exclude_with_wildcard_include() {
        let f = ModelFilter::new(vec!["*".into()], vec!["ir.*".into()]);
        assert!(f.matches("res.partner"));
        assert!(!f.matches("ir.model"));
    }

    #[test]
    fn filter_json_models() {
        let f = ModelFilter::new(vec!["res.*".into()], vec![]);
        let models = vec![
            json!({"model": "res.partner", "name": "Contact"}),
            json!({"model": "sale.order", "name": "Sale Order"}),
            json!({"model": "res.users", "name": "Users"}),
        ];
        let result = f.filter(models);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["model"], "res.partner");
        assert_eq!(result[1]["model"], "res.users");
    }

    #[test]
    fn filter_skips_entries_without_model_field() {
        let f = ModelFilter::new(vec![], vec![]);
        let models = vec![json!({"name": "No model field"})];
        let result = f.filter(models);
        assert!(result.is_empty());
    }
}
