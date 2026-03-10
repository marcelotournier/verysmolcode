use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTier {
    Pro,
    Flash,
    FlashLite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum ModelId {
    // Gemini 3.x (newest, preferred)
    Gemini31Pro,
    Gemini3Flash,
    Gemini31FlashLite,
    // Gemini 2.5 (stable fallbacks)
    Gemini25Pro,
    Gemini25Flash,
    Gemini25FlashLite,
}

impl ModelId {
    pub fn api_name(&self) -> &'static str {
        match self {
            ModelId::Gemini31Pro => "gemini-3.1-pro-preview",
            ModelId::Gemini3Flash => "gemini-3-flash-preview",
            ModelId::Gemini31FlashLite => "gemini-3.1-flash-lite-preview",
            ModelId::Gemini25Pro => "gemini-2.5-pro",
            ModelId::Gemini25Flash => "gemini-2.5-flash",
            ModelId::Gemini25FlashLite => "gemini-2.5-flash-lite",
        }
    }

    pub fn tier(&self) -> ModelTier {
        match self {
            ModelId::Gemini31Pro | ModelId::Gemini25Pro => ModelTier::Pro,
            ModelId::Gemini3Flash | ModelId::Gemini25Flash => ModelTier::Flash,
            ModelId::Gemini31FlashLite | ModelId::Gemini25FlashLite => ModelTier::FlashLite,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ModelId::Gemini31Pro => "Gemini 3.1 Pro",
            ModelId::Gemini3Flash => "Gemini 3 Flash",
            ModelId::Gemini31FlashLite => "Gemini 3.1 Flash-Lite",
            ModelId::Gemini25Pro => "Gemini 2.5 Pro",
            ModelId::Gemini25Flash => "Gemini 2.5 Flash",
            ModelId::Gemini25FlashLite => "Gemini 2.5 Flash-Lite",
        }
    }

    pub fn supports_thinking(&self) -> bool {
        // All models support thinking via thinkingConfig
        // Gemini 2.5 Pro added thinking support, 3.x models all have it
        matches!(
            self,
            ModelId::Gemini3Flash
                | ModelId::Gemini31FlashLite
                | ModelId::Gemini31Pro
                | ModelId::Gemini25Pro
                | ModelId::Gemini25Flash
                | ModelId::Gemini25FlashLite
        )
    }

    /// All known model IDs
    pub fn all() -> &'static [ModelId] {
        &[
            ModelId::Gemini31Pro,
            ModelId::Gemini3Flash,
            ModelId::Gemini31FlashLite,
            ModelId::Gemini25Pro,
            ModelId::Gemini25Flash,
            ModelId::Gemini25FlashLite,
        ]
    }
}

#[derive(Debug, Clone)]
pub struct RateLimit {
    pub rpm: u32, // requests per minute
    pub rpd: u32, // requests per day
    pub tpm: u32, // tokens per minute
}

impl RateLimit {
    pub fn for_model(model: ModelId) -> Self {
        match model.tier() {
            ModelTier::Pro => RateLimit {
                rpm: 5,
                rpd: 25,
                tpm: 250_000,
            },
            ModelTier::Flash => RateLimit {
                rpm: 10,
                rpd: 250,
                tpm: 1_000_000,
            },
            ModelTier::FlashLite => RateLimit {
                rpm: 15,
                rpd: 1000,
                tpm: 1_000_000,
            },
        }
    }
}

#[derive(Debug)]
pub struct RateLimiter {
    model: ModelId,
    limits: RateLimit,
    minute_requests: Vec<Instant>,
    day_requests: Vec<Instant>,
}

impl RateLimiter {
    pub fn new(model: ModelId) -> Self {
        Self {
            model,
            limits: RateLimit::for_model(model),
            minute_requests: Vec::new(),
            day_requests: Vec::new(),
        }
    }

    fn cleanup(&mut self) {
        let now = Instant::now();
        let one_minute = Duration::from_secs(60);
        let one_day = Duration::from_secs(86400);
        self.minute_requests
            .retain(|t| now.duration_since(*t) < one_minute);
        self.day_requests
            .retain(|t| now.duration_since(*t) < one_day);
    }

    pub fn can_request(&mut self) -> bool {
        self.cleanup();
        self.minute_requests.len() < self.limits.rpm as usize
            && self.day_requests.len() < self.limits.rpd as usize
    }

    pub fn record_request(&mut self) {
        let now = Instant::now();
        self.minute_requests.push(now);
        self.day_requests.push(now);
    }

    pub fn wait_duration(&mut self) -> Option<Duration> {
        self.cleanup();
        if self.day_requests.len() >= self.limits.rpd as usize {
            return None; // Daily limit exhausted, cannot wait
        }
        if self.minute_requests.len() >= self.limits.rpm as usize {
            if let Some(oldest) = self.minute_requests.first() {
                let elapsed = Instant::now().duration_since(*oldest);
                let wait = Duration::from_secs(60).saturating_sub(elapsed);
                return Some(wait);
            }
        }
        Some(Duration::ZERO)
    }

    pub fn remaining_today(&mut self) -> u32 {
        self.cleanup();
        self.limits
            .rpd
            .saturating_sub(self.day_requests.len() as u32)
    }

    pub fn model(&self) -> ModelId {
        self.model
    }
}

/// Manages rate limiters for all models and picks the best available.
/// Gemini 3 models are preferred; falls back to 2.5 when rate-limited.
#[derive(Debug)]
pub struct ModelRouter {
    // Gemini 3.x
    pub g3_pro: RateLimiter,
    pub g3_flash: RateLimiter,
    pub g3_flash_lite: RateLimiter,
    // Gemini 2.5
    pub pro: RateLimiter,
    pub flash: RateLimiter,
    pub flash_lite: RateLimiter,
}

impl Default for ModelRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelRouter {
    pub fn new() -> Self {
        Self {
            g3_pro: RateLimiter::new(ModelId::Gemini31Pro),
            g3_flash: RateLimiter::new(ModelId::Gemini3Flash),
            g3_flash_lite: RateLimiter::new(ModelId::Gemini31FlashLite),
            pro: RateLimiter::new(ModelId::Gemini25Pro),
            flash: RateLimiter::new(ModelId::Gemini25Flash),
            flash_lite: RateLimiter::new(ModelId::Gemini25FlashLite),
        }
    }

    fn limiter_mut(&mut self, model: ModelId) -> &mut RateLimiter {
        match model {
            ModelId::Gemini31Pro => &mut self.g3_pro,
            ModelId::Gemini3Flash => &mut self.g3_flash,
            ModelId::Gemini31FlashLite => &mut self.g3_flash_lite,
            ModelId::Gemini25Pro => &mut self.pro,
            ModelId::Gemini25Flash => &mut self.flash,
            ModelId::Gemini25FlashLite => &mut self.flash_lite,
        }
    }

    /// Pick the best available model for a task complexity level.
    /// Prefers Gemini 3 models, falls back to 2.5.
    /// Returns None if all models are exhausted for the day.
    pub fn pick_model(&mut self, prefer_smart: bool) -> Option<ModelId> {
        if prefer_smart {
            // Pro tier first (3.1 Pro -> 2.5 Pro), then Flash, then Lite
            let order = [
                ModelId::Gemini31Pro,
                ModelId::Gemini25Pro,
                ModelId::Gemini3Flash,
                ModelId::Gemini25Flash,
                ModelId::Gemini31FlashLite,
                ModelId::Gemini25FlashLite,
            ];
            for model in order {
                if self.limiter_mut(model).can_request() {
                    return Some(model);
                }
            }
        } else {
            // Flash first (save Pro for hard tasks)
            let order = [
                ModelId::Gemini3Flash,
                ModelId::Gemini25Flash,
                ModelId::Gemini31FlashLite,
                ModelId::Gemini25FlashLite,
                ModelId::Gemini31Pro,
                ModelId::Gemini25Pro,
            ];
            for model in order {
                if self.limiter_mut(model).can_request() {
                    return Some(model);
                }
            }
        }
        None
    }

    pub fn record_request(&mut self, model: ModelId) {
        self.limiter_mut(model).record_request();
    }

    pub fn wait_for_model(&mut self, model: ModelId) -> Option<Duration> {
        self.limiter_mut(model).wait_duration()
    }

    /// Get the next fallback model after a failure
    pub fn fallback_for(&mut self, model: ModelId) -> Option<ModelId> {
        // Try same tier but different generation, then lower tiers
        let fallback_order: &[ModelId] = match model {
            ModelId::Gemini31Pro => &[
                ModelId::Gemini25Pro,
                ModelId::Gemini3Flash,
                ModelId::Gemini25Flash,
                ModelId::Gemini31FlashLite,
                ModelId::Gemini25FlashLite,
            ],
            ModelId::Gemini25Pro => &[
                ModelId::Gemini3Flash,
                ModelId::Gemini25Flash,
                ModelId::Gemini31FlashLite,
                ModelId::Gemini25FlashLite,
            ],
            ModelId::Gemini3Flash => &[
                ModelId::Gemini25Flash,
                ModelId::Gemini31FlashLite,
                ModelId::Gemini25FlashLite,
            ],
            ModelId::Gemini25Flash => &[ModelId::Gemini31FlashLite, ModelId::Gemini25FlashLite],
            ModelId::Gemini31FlashLite => &[ModelId::Gemini25FlashLite],
            ModelId::Gemini25FlashLite => &[],
        };

        fallback_order
            .iter()
            .copied()
            .find(|&fb| self.limiter_mut(fb).can_request())
    }

    pub fn status_line(&mut self) -> String {
        format!(
            "3Pro:{} 3F:{} 3L:{} | Pro:{} F:{} L:{}",
            self.g3_pro.remaining_today(),
            self.g3_flash.remaining_today(),
            self.g3_flash_lite.remaining_today(),
            self.pro.remaining_today(),
            self.flash.remaining_today(),
            self.flash_lite.remaining_today(),
        )
    }
}
