import { PublicKey } from "@solana/web3.js";

/**
 * Validate a Solana public key string
 */
export function validatePublicKey(value: string): boolean | string {
    if (!value || value.trim() === "") {
        return "Public key is required";
    }
    try {
        new PublicKey(value.trim());
        return true;
    } catch {
        return "Invalid public key format";
    }
}

/**
 * Validate basis points (0-10000)
 */
export function validateBasisPoints(value: number): boolean | string {
    if (value === undefined || value === null || isNaN(value)) {
        return "Basis points value is required";
    }
    if (value < 0 || value > 10000) {
        return "Basis points must be between 0 and 10000";
    }
    if (!Number.isInteger(value)) {
        return "Basis points must be a whole number";
    }
    return true;
}

/**
 * Validate a positive integer amount
 */
export function validateAmount(value: number): boolean | string {
    if (value === undefined || value === null || isNaN(value)) {
        return "Amount is required";
    }
    if (value <= 0) {
        return "Amount must be positive";
    }
    if (!Number.isInteger(value)) {
        return "Amount must be a whole number";
    }
    return true;
}

/**
 * Validate APR value (scaled by 1_000_000)
 */
export function validateApr(value: number): boolean | string {
    if (value === undefined || value === null || isNaN(value)) {
        return "APR is required";
    }
    if (value < 0) {
        return "APR cannot be negative";
    }
    return true;
}

/**
 * Validate timestamp (Unix seconds or ISO date string)
 */
export function validateTimestamp(value: string | number): boolean | string {
    if (value === undefined || value === null || value === "") {
        return "Timestamp is required";
    }

    if (typeof value === "string") {
        if (value.toLowerCase() === "now") {
            return true;
        }
        const date = new Date(value);
        if (isNaN(date.getTime())) {
            return "Invalid date format. Use ISO format (e.g., 2025-06-01T00:00:00Z) or 'now'";
        }
    } else if (typeof value === "number") {
        if (value < 0) {
            return "Timestamp cannot be negative";
        }
    }

    return true;
}

/**
 * Validate duration in seconds
 */
export function validateDuration(value: number): boolean | string {
    if (value === undefined || value === null || isNaN(value)) {
        return "Duration is required";
    }
    if (value <= 0) {
        return "Duration must be positive";
    }
    if (!Number.isInteger(value)) {
        return "Duration must be a whole number of seconds";
    }
    return true;
}

/**
 * Parse a timestamp string/number into Unix seconds
 */
export function parseTimestamp(value: string | number): number {
    if (typeof value === "number") {
        return value;
    }
    if (value.toLowerCase() === "now") {
        return Math.floor(Date.now() / 1000);
    }
    return Math.floor(new Date(value).getTime() / 1000);
}
