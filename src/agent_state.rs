//! Explicit, typed projections of agent-owned Onto state. Declarations do not
//! grant write authority: existing Onto Stories remain the only mutation path.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Declaration {
    pub fields: Vec<Field>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub key: String,
    pub label: String,
    pub kind: Kind,
    pub format: Format,
    pub source: Source,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub instance: String,
    pub field: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    String,
    Integer,
    Number,
    Boolean,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Plain,
    UsdCents,
}

impl Declaration {
    pub fn identity(actor: &str) -> Self {
        Self {
            fields: vec![Field {
                key: "name".into(),
                label: "Name".into(),
                kind: Kind::String,
                format: Format::Plain,
                source: Source {
                    instance: actor.into(),
                    field: "name".into(),
                },
            }],
        }
    }

    /// Validate against a candidate snapshot before publishing either part.
    pub fn project(&self, actor: &str, snapshot: &Value) -> Result<Value> {
        ensure!(
            self.fields.len() <= 24,
            "Agent state allows at most 24 fields per scope"
        );
        let instances = snapshot["instances"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Missing agent-state snapshot"))?;
        let mut keys = BTreeSet::new();
        let mut values = serde_json::Map::new();
        for field in &self.fields {
            ensure!(
                !field.key.is_empty()
                    && field.key.len() <= 64
                    && field
                        .key
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
                    && keys.insert(&field.key),
                "Agent-state field keys must be unique identifiers"
            );
            ensure!(
                !field.label.trim().is_empty()
                    && field.label.len() <= 120
                    && field.source.instance.len() <= 200
                    && !field.source.field.is_empty()
                    && field.source.field.len() <= 100,
                "Invalid agent-state field metadata"
            );
            let record = instances
                .iter()
                .find(|record| record["id"] == field.source.instance)
                .ok_or_else(|| {
                    anyhow::anyhow!("Agent-state record {} is missing", field.source.instance)
                })?;
            ensure!(
                record["id"] == actor || record["fields"]["owner"] == actor,
                "Agent-state record {} is not owned by {actor}",
                field.source.instance
            );
            let value = &record["fields"][&field.source.field];
            let matches = match field.kind {
                Kind::String => value.is_string(),
                Kind::Integer => value.is_i64() || value.is_u64(),
                Kind::Number => value.is_number(),
                Kind::Boolean => value.is_boolean(),
            };
            ensure!(
                matches,
                "Agent-state field {} is missing or has the wrong type",
                field.key
            );
            ensure!(
                field.format != Format::UsdCents || field.kind == Kind::Integer,
                "USD cents require an integer agent-state field"
            );
            values.insert(field.key.clone(), value.clone());
        }
        Ok(json!({"shape":self,"values":values}))
    }
}

pub fn schema() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["fields"],"properties":{"fields":{"type":"array","items":{"type":"object","additionalProperties":false,"required":["key","label","kind","format","source"],"properties":{"key":{"type":"string"},"label":{"type":"string"},"kind":{"type":"string","enum":["string","integer","number","boolean"]},"format":{"type":"string","enum":["plain","usd_cents"]},"source":{"type":"object","additionalProperties":false,"required":["instance","field"],"properties":{"instance":{"type":"string"},"field":{"type":"string"}}}}}}}})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn declarations_bind_owned_fields_and_reject_invalid_updates() -> Result<()> {
        let shape: Declaration = serde_json::from_value(
            json!({"fields":[{"key":"budget","label":"Available","kind":"integer","format":"usd_cents","source":{"instance":"Wallet:mine","field":"balance"}}]}),
        )?;
        let snapshot = json!({"instances":[{"id":"Wallet:mine","fields":{"owner":"Operator:Ada","balance":0}}]});
        assert_eq!(
            shape.project("Operator:Ada", &snapshot)?["values"]["budget"],
            0
        );
        assert!(shape.project("Operator:Other", &snapshot).is_err());
        for value in [Value::Null, json!("100"), json!(1.5)] {
            let mut invalid = snapshot.clone();
            invalid["instances"][0]["fields"]["balance"] = value;
            assert!(shape.project("Operator:Ada", &invalid).is_err());
        }
        Ok(())
    }
}
