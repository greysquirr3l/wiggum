//! Model pricing data for cost estimation.

use serde::{Deserialize, Serialize};

/// Pricing information for a single model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub name: String,
    /// Cost per 1M input tokens in USD.
    pub input_per_m: f64,
    /// Cost per 1M output tokens in USD.
    pub output_per_m: f64,
}

/// Collection of model pricing data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingData {
    /// Last updated timestamp (ISO 8601).
    pub last_updated: String,
    pub models: Vec<ModelPricing>,
}

impl Default for PricingData {
    fn default() -> Self {
        Self::bundled()
    }
}

impl PricingData {
    /// Bundled pricing data (March 2026).
    #[must_use]
    pub fn bundled() -> Self {
        Self {
            last_updated: "2026-03".to_string(),
            models: vec![
                ModelPricing {
                    name: "claude-sonnet-4".to_string(),
                    input_per_m: 3.0,
                    output_per_m: 15.0,
                },
                ModelPricing {
                    name: "claude-haiku-3.5".to_string(),
                    input_per_m: 0.8,
                    output_per_m: 4.0,
                },
                ModelPricing {
                    name: "gpt-4o".to_string(),
                    input_per_m: 5.0,
                    output_per_m: 15.0,
                },
                ModelPricing {
                    name: "gpt-4o-mini".to_string(),
                    input_per_m: 0.15,
                    output_per_m: 0.6,
                },
                ModelPricing {
                    name: "gemini-2.0-flash".to_string(),
                    input_per_m: 0.1,
                    output_per_m: 0.4,
                },
            ],
        }
    }

    /// Get pricing for a specific model by name (case-insensitive).
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&ModelPricing> {
        let lower = name.to_lowercase();
        self.models.iter().find(|m| m.name.to_lowercase() == lower)
    }

    /// Estimate cost for a given token count.
    ///
    /// Assumes output tokens = 2× input tokens (conservative).
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // Token counts are well within f64 precision
    pub fn estimate_cost(&self, input_tokens: usize) -> Vec<CostEstimate> {
        let input = input_tokens as f64;
        let output = input * 2.0; // Conservative 2x multiplier
        let per_m = 1_000_000.0_f64;

        self.models
            .iter()
            .map(|m| {
                let cost =
                    (input / per_m).mul_add(m.input_per_m, (output / per_m) * m.output_per_m);
                CostEstimate {
                    model: m.name.clone(),
                    cost_usd: cost,
                }
            })
            .collect()
    }
}

/// Estimated cost for a single model.
#[derive(Debug, Clone)]
pub struct CostEstimate {
    pub model: String,
    pub cost_usd: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_data_has_models() {
        let data = PricingData::bundled();
        assert!(!data.models.is_empty());
    }

    #[test]
    fn estimate_cost_basic() {
        let data = PricingData::bundled();
        let estimates = data.estimate_cost(100_000);
        assert!(!estimates.is_empty());
        for est in estimates {
            assert!(est.cost_usd > 0.0);
        }
    }
}
