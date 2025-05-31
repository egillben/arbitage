//! Utilities Module
//!
//! This module contains utility functions for the MEV arbitrage bot.

use anyhow::Result;
use ethers::types::{Address, U256};
use log::{debug, error, info, warn};
use std::str::FromStr;
use std::time::{Duration, Instant};

/// Convert a decimal number to a U256 with the specified number of decimals
pub fn decimal_to_u256(amount: f64, decimals: u8) -> U256 {
    let factor = 10u64.pow(decimals as u32);
    let integer_part = amount.trunc() as u64;
    let fractional_part = ((amount.fract() * factor as f64).round() as u64) % factor;

    let integer_part_u256 = U256::from(integer_part).saturating_mul(U256::from(factor));
    let fractional_part_u256 = U256::from(fractional_part);

    integer_part_u256.saturating_add(fractional_part_u256)
}

/// Convert a U256 to a decimal number with the specified number of decimals
pub fn u256_to_decimal(amount: U256, decimals: u8) -> f64 {
    let factor = 10u64.pow(decimals as u32);
    let factor_u256 = U256::from(factor);

    let integer_part = amount.checked_div(factor_u256).unwrap_or_default();
    let fractional_part = amount.saturating_sub(integer_part.saturating_mul(factor_u256));

    let integer_part_f64 = integer_part.as_u128() as f64;
    let fractional_part_f64 = fractional_part.as_u128() as f64 / factor as f64;

    integer_part_f64 + fractional_part_f64
}

/// Format a U256 as a decimal string with the specified number of decimals
pub fn format_u256(amount: U256, decimals: u8) -> String {
    let decimal = u256_to_decimal(amount, decimals);
    format!("{:.6}", decimal)
}

/// Validates and normalizes an Ethereum address string before parsing it
///
/// This function ensures that:
/// 1. The address has the "0x" prefix
/// 2. The address has exactly 40 hex characters after the prefix
/// 3. The address contains only valid hex characters
///
/// # Arguments
///
/// * `address_str` - A string slice containing the Ethereum address to validate and parse
///
/// # Returns
///
/// * `Result<Address>` - The parsed Ethereum address or an error if validation fails
pub fn validate_and_parse_address(address_str: &str) -> Result<Address> {
    // Trim whitespace
    let trimmed = address_str.trim();

    // Normalize the address: ensure it has 0x prefix and is lowercase
    let normalized = if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
        format!("0x{}", &trimmed[2..].to_lowercase())
    } else {
        format!("0x{}", trimmed.to_lowercase())
    };

    // Check if the address has the correct length (0x + 40 hex chars)
    if normalized.len() != 42 {
        anyhow::bail!(
            "Invalid Ethereum address length: expected 42 characters (including 0x prefix), got {}",
            normalized.len()
        );
    }

    // Check if the address contains only valid hex characters after the 0x prefix
    if !normalized[2..].chars().all(|c| c.is_digit(16)) {
        anyhow::bail!("Invalid Ethereum address: contains non-hexadecimal characters");
    }

    // Parse the normalized address
    Address::from_str(&normalized)
        .map_err(|e| anyhow::anyhow!("Failed to parse Ethereum address: {}", e))
}

/// Parse an address from a string (legacy function, use validate_and_parse_address instead)
pub fn parse_address(address: &str) -> Result<Address> {
    Ok(Address::from_str(address)?)
}

/// Calculate the price impact of a trade
pub fn calculate_price_impact(
    input_amount: U256,
    input_price: f64,
    output_amount: U256,
    output_price: f64,
) -> f64 {
    let input_value = u256_to_decimal(input_amount, 18) * input_price;
    let output_value = u256_to_decimal(output_amount, 18) * output_price;

    if input_value == 0.0 {
        return 0.0;
    }

    let price_impact = (input_value - output_value) / input_value * 100.0;
    price_impact.max(0.0)
}

/// Calculate the profit of a trade
pub fn calculate_profit(
    input_amount: U256,
    input_price: f64,
    output_amount: U256,
    output_price: f64,
) -> f64 {
    let input_value = u256_to_decimal(input_amount, 18) * input_price;
    let output_value = u256_to_decimal(output_amount, 18) * output_price;

    output_value - input_value
}

/// Calculate the gas cost in USD
pub fn calculate_gas_cost(gas_used: U256, gas_price: U256, eth_price: f64) -> f64 {
    let gas_cost_eth = u256_to_decimal(gas_used.saturating_mul(gas_price), 18);
    gas_cost_eth * eth_price
}

/// Generate a unique ID
pub fn generate_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let id: u64 = rng.gen();
    format!("{:016x}", id)
}

/// Measure the execution time of a function
pub async fn measure_time<F, T>(f: F) -> (T, Duration)
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let result = f();
    let duration = start.elapsed();
    (result, duration)
}

/// Measure the execution time of an async function
pub async fn measure_time_async<F, Fut, T>(f: F) -> (T, Duration)
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let start = Instant::now();
    let result = f().await;
    let duration = start.elapsed();
    (result, duration)
}

/// Retry a function with exponential backoff
pub async fn retry_with_backoff<F, Fut, T, E>(
    f: F,
    max_retries: u32,
    initial_backoff: Duration,
) -> Result<T, E>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Debug,
{
    let mut retries = 0;
    let mut backoff = initial_backoff;

    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                if retries >= max_retries {
                    return Err(e);
                }

                warn!(
                    "Retry {}/{} failed: {:?}. Retrying in {:?}...",
                    retries + 1,
                    max_retries,
                    e,
                    backoff
                );

                tokio::time::sleep(backoff).await;

                retries += 1;
                backoff = backoff.saturating_mul(2);
            }
        }
    }
}

/// Truncate a string to a maximum length
pub fn truncate_string(s: &str, max_length: usize) -> String {
    if s.len() <= max_length {
        s.to_string()
    } else {
        format!("{}...", &s[0..max_length - 3])
    }
}

/// Format a duration as a human-readable string
pub fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    let millis = duration.subsec_millis();

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else if seconds > 0 {
        format!("{}s {}ms", seconds, millis)
    } else {
        format!("{}ms", millis)
    }
}

/// Format a timestamp as a human-readable string
pub fn format_timestamp(timestamp: u64) -> String {
    use chrono::{DateTime, NaiveDateTime, Utc};

    let naive = NaiveDateTime::from_timestamp_opt(timestamp as i64, 0).unwrap_or_default();
    let datetime: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive, Utc);

    datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

/// Create a directory if it doesn't exist
pub fn create_directory_if_not_exists(path: &str) -> Result<()> {
    use std::fs;

    if !std::path::Path::new(path).exists() {
        fs::create_dir_all(path)?;
        debug!("Created directory: {}", path);
    }

    Ok(())
}

/// Write data to a file
pub fn write_to_file(path: &str, data: &str) -> Result<()> {
    use std::fs;
    use std::io::Write;

    // Create the directory if it doesn't exist
    if let Some(parent) = std::path::Path::new(path).parent() {
        create_directory_if_not_exists(parent.to_str().unwrap_or("."))?;
    }

    // Write the data to the file
    let mut file = fs::File::create(path)?;
    file.write_all(data.as_bytes())?;

    debug!("Wrote data to file: {}", path);

    Ok(())
}

/// Read data from a file
pub fn read_from_file(path: &str) -> Result<String> {
    use std::fs;

    let data = fs::read_to_string(path)?;

    debug!("Read data from file: {}", path);

    Ok(data)
}

/// Check if a file exists
pub fn file_exists(path: &str) -> bool {
    std::path::Path::new(path).exists()
}

/// Get the current timestamp in seconds
pub fn current_timestamp() -> u64 {
    use std::time::SystemTime;

    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Get the current timestamp in milliseconds
pub fn current_timestamp_millis() -> u128 {
    use std::time::SystemTime;

    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
