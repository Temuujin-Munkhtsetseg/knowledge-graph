// Retry utilities with exponential backoff and configurable strategies
export interface RetryOptions {
  maxAttempts: number;
  initialDelay: number;
  maxDelay?: number;
  exponentialBackoff?: boolean;
  backoffFactor?: number;
  shouldRetry?: (error: Error, attempt: number) => boolean;
  onRetry?: (error: Error, attempt: number) => void;
}

export interface RetryConfig extends Partial<RetryOptions> {
  maxAttempts?: number;
  initialDelay?: number;
}

// Default retry configuration
export const DEFAULT_RETRY_CONFIG: Required<RetryOptions> = {
  maxAttempts: 3,
  initialDelay: 1000,
  maxDelay: 30000,
  exponentialBackoff: true,
  backoffFactor: 2,
  shouldRetry: (error: Error) => {
    // Default retry logic - retry on network errors and 5xx status codes
    if (error.name === 'TypeError' && error.message.includes('fetch')) {
      return true; // Network error
    }

    if ('status' in error && typeof error.status === 'number') {
      return error.status >= 500 && error.status < 600; // 5xx errors
    }

    return false;
  },
  onRetry: () => {}, // No-op by default
};

// Error classifications for retry decisions
export enum RetryableErrorType {
  NetworkError = 'network',
  ServerError = 'server',
  TimeoutError = 'timeout',
  RateLimitError = 'rate_limit',
  Unknown = 'unknown',
}

export function classifyError(error: Error): RetryableErrorType {
  // Network/fetch errors
  if (error.name === 'TypeError' && error.message.includes('fetch')) {
    return RetryableErrorType.NetworkError;
  }

  // Timeout errors
  if (error.name === 'AbortError' || error.message.includes('timeout')) {
    return RetryableErrorType.TimeoutError;
  }

  // Status-based classification
  if ('status' in error && typeof error.status === 'number') {
    const { status } = error as { status: number };

    if (status === 429) {
      return RetryableErrorType.RateLimitError;
    }

    if (status >= 500 && status < 600) {
      return RetryableErrorType.ServerError;
    }
  }

  return RetryableErrorType.Unknown;
}

// Smart retry strategy based on error type
export function createSmartRetryStrategy(): (error: Error, attempt: number) => boolean {
  return (error: Error, attempt: number) => {
    const errorType = classifyError(error);

    switch (errorType) {
      case RetryableErrorType.NetworkError:
        return attempt <= 5; // More retries for network issues

      case RetryableErrorType.ServerError:
        return attempt <= 3; // Standard retries for server errors

      case RetryableErrorType.TimeoutError:
        return attempt <= 2; // Fewer retries for timeouts

      case RetryableErrorType.RateLimitError:
        return attempt <= 1; // Very limited retries for rate limits

      default:
        return false; // No retries for unknown errors
    }
  };
}

// Calculate delay with exponential backoff and jitter
export function calculateDelay(attempt: number, options: Required<RetryOptions>): number {
  let delay = options.initialDelay;

  if (options.exponentialBackoff) {
    delay = options.initialDelay * options.backoffFactor ** (attempt - 1);
  }

  // Apply maximum delay cap
  delay = Math.min(delay, options.maxDelay);

  // Add jitter to prevent thundering herd
  const jitter = Math.random() * 0.1 * delay;
  return Math.floor(delay + jitter);
}

// Main retry function with comprehensive error handling
export async function withRetry<T>(
  operation: () => Promise<T>,
  config: RetryConfig = {},
): Promise<T> {
  const options: Required<RetryOptions> = {
    ...DEFAULT_RETRY_CONFIG,
    ...config,
  };

  let lastError: Error | undefined;

  for (let attempt = 1; attempt <= options.maxAttempts; attempt++) {
    try {
      // eslint-disable-next-line no-await-in-loop
      return await operation();
    } catch (error) {
      lastError = error as Error;

      // Don't retry if this is the last attempt
      if (attempt === options.maxAttempts) {
        throw lastError;
      }

      // Check if we should retry this error
      if (!options.shouldRetry(lastError, attempt)) {
        throw lastError;
      }

      // Notify about retry
      options.onRetry(lastError, attempt);

      // Calculate and wait for delay
      const delay = calculateDelay(attempt, options);
      // eslint-disable-next-line no-await-in-loop
      await sleep(delay);
    }
  }

  // This should never be reached due to the logic above, but TypeScript requires it
  throw new Error((lastError as unknown as Error)?.message || 'Operation failed after retries');
}

// Promise-based sleep utility
export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}

// Specialized retry functions for common patterns
export async function retryFetch(
  url: string,
  init?: RequestInit,
  retryConfig?: RetryConfig,
): Promise<Response> {
  return withRetry(() => fetch(url, init), {
    maxAttempts: 3,
    initialDelay: 1000,
    shouldRetry: (error) => {
      const errorType = classifyError(error);
      return (
        errorType === RetryableErrorType.NetworkError ||
        errorType === RetryableErrorType.ServerError
      );
    },
    ...retryConfig,
  });
}

export async function retryOperation<T>(
  operation: () => Promise<T>,
  maxRetries: number = 3,
  baseDelay: number = 1000,
): Promise<T> {
  return withRetry(operation, {
    maxAttempts: maxRetries,
    initialDelay: baseDelay,
    exponentialBackoff: true,
  });
}

// Retry handler class for stateful retry management
export class RetryHandler {
  #config: Required<RetryOptions>;

  constructor(config: RetryConfig = {}) {
    this.#config = {
      ...DEFAULT_RETRY_CONFIG,
      ...config,
    };
  }

  async execute<T>(operation: () => Promise<T>): Promise<T> {
    return withRetry(operation, this.#config);
  }

  updateConfig(newConfig: Partial<RetryOptions>): void {
    Object.assign(this.#config, newConfig);
  }

  getConfig(): Readonly<Required<RetryOptions>> {
    return { ...this.#config };
  }
}

// Export common retry configurations
export const RETRY_CONFIGS = {
  // Conservative retry for critical operations
  conservative: {
    maxAttempts: 2,
    initialDelay: 2000,
    exponentialBackoff: false,
  },

  // Standard retry for most operations
  standard: {
    maxAttempts: 3,
    initialDelay: 1000,
    exponentialBackoff: true,
  },

  // Aggressive retry for non-critical operations
  aggressive: {
    maxAttempts: 5,
    initialDelay: 500,
    exponentialBackoff: true,
    maxDelay: 10000,
  },

  // Smart retry with error-based strategy
  smart: {
    maxAttempts: 4,
    initialDelay: 1000,
    exponentialBackoff: true,
    shouldRetry: createSmartRetryStrategy(),
  },
} as const;
