use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTier {
    Pro,
    Flash,
    FlashLite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelId {
    Gemini25Pro,
    Gemini25Flash,
    Gemini25FlashLite,
}

impl ModelId {
    pub fn api_name(&self) -> &'static str {
        match self {
            ModelId::Gemini25Pro => "gemini-2.5-pro-preview-05-06",
            ModelId::Gemini25Flash => "gemini-2.5-flash-preview-05-20",
            ModelId::Gemini25FlashLite => "gemini-2.0-flash-lite",
        }
    }

    pub fn tier(&self) -> ModelTier {
        match self {
            ModelId::Gemini25Pro => ModelTier::Pro,
            ModelId::Gemini25Flash => ModelTier::Flash,
            ModelId::Gemini25FlashLite => ModelTier::FlashLite,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ModelId::Gemini25Pro => "Gemini 2.5 Pro",
            ModelId::Gemini25Flash => "Gemini 2.5 Flash",
            ModelId::Gemini25FlashLite => "Gemini 2.0 Flash-Lite",
        }
    }

    pub fn supports_thinking(&self) -> bool {
        matches!(self, ModelId::Gemini25Flash | ModelId::Gemini25FlashLite)
    }
}

#[derive(Debug, Clone)]
pub struct RateLimit {
    pub rpm: u32,        // requests per minute
    pub rpd: u32,        // requests per day
    pub tpm: u32,        // tokens per minute
}

impl RateLimit {
    pub fn for_model(model: ModelId) -> Self {
        match model.tier() {
            ModelTier::Pro => RateLimit { rpm: 5, rpd: 25, tpm: 250_000 },
            ModelTier::Flash => RateLimit { rpm: 10, rpd: 250, tpm: 1_000_000 },
            ModelTier::FlashLite => RateLimit { rpm: 15, rpd: 1000, tpm: 1_000_000 },
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
        self.minute_requests.retain(|t| now.duration_since(*t) < one_minute);
        self.day_requests.retain(|t| now.duration_since(*t) < one_day);
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
        self.limits.rpd.saturating_sub(self.day_requests.len() as u32)
    }

    pub fn model(&self) -> ModelId {
        self.model
    }
}

/// Manages rate limiters for all models and picks the best available
#[derive(Debug)]
pub struct ModelRouter {
    pub pro: RateLimiter,
    pub flash: RateLimiter,
    pub flash_lite: RateLimiter,
}

impl ModelRouter {
    pub fn new() -> Self {
        Self {
            pro: RateLimiter::new(ModelId::Gemini25Pro),
            flash: RateLimiter::new(ModelId::Gemini25Flash),
            flash_lite: RateLimiter::new(ModelId::Gemini25FlashLite),
        }
    }

    /// Pick the best available model for a task complexity level.
    /// Returns None if all models are exhausted for the day.
    pub fn pick_model(&mut self, prefer_smart: bool) -> Option<ModelId> {
        if prefer_smart {
            // Try Pro -> Flash -> Flash-Lite
            if self.pro.can_request() {
                return Some(ModelId::Gemini25Pro);
            }
            if self.flash.can_request() {
                return Some(ModelId::Gemini25Flash);
            }
            if self.flash_lite.can_request() {
                return Some(ModelId::Gemini25FlashLite);
            }
        } else {
            // Try Flash -> Flash-Lite -> Pro (save Pro for hard tasks)
            if self.flash.can_request() {
                return Some(ModelId::Gemini25Flash);
            }
            if self.flash_lite.can_request() {
                return Some(ModelId::Gemini25FlashLite);
            }
            if self.pro.can_request() {
                return Some(ModelId::Gemini25Pro);
            }
        }
        None
    }

    pub fn record_request(&mut self, model: ModelId) {
        match model {
            ModelId::Gemini25Pro => self.pro.record_request(),
            ModelId::Gemini25Flash => self.flash.record_request(),
            ModelId::Gemini25FlashLite => self.flash_lite.record_request(),
        }
    }

    pub fn wait_for_model(&mut self, model: ModelId) -> Option<Duration> {
        match model {
            ModelId::Gemini25Pro => self.pro.wait_duration(),
            ModelId::Gemini25Flash => self.flash.wait_duration(),
            ModelId::Gemini25FlashLite => self.flash_lite.wait_duration(),
        }
    }

    pub fn status_line(&mut self) -> String {
        format!(
            "Pro: {}/day | Flash: {}/day | Lite: {}/day",
            self.pro.remaining_today(),
            self.flash.remaining_today(),
            self.flash_lite.remaining_today(),
        )
    }
}
