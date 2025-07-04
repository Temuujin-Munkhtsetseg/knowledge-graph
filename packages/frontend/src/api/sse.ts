import { createParser, type EventSourceMessage } from 'eventsource-parser';
import { withRetry } from './retry-handler';
import { createTextStreamReader, type StreamReader } from './stream-reader';
import type { BaseEventCallbacks } from './types';

export interface SSEConnectionOptions extends BaseEventCallbacks {
  onEvent: (event: EventSourceMessage) => void;
  enableAutoReconnect?: boolean;
  maxRetryAttempts?: number;
  retryDelay?: number;
}

export class SSEConnection {
  #abortController: AbortController | null = null;

  #streamReader: StreamReader | null = null;

  #isConnected = false;

  #callbacks: BaseEventCallbacks = {};

  async connect(url: string, options: SSEConnectionOptions): Promise<void> {
    if (this.#isConnected) {
      throw new Error('SSE connection is already active.');
    }

    this.#abortController = new AbortController();
    this.#isConnected = true;
    this.#callbacks = options;

    const connectAndRead = async () => {
      try {
        const response = await fetch(url, {
          headers: {
            Accept: 'text/event-stream',
            'Cache-Control': 'no-cache',
          },
          signal: this.#abortController?.signal,
        });

        if (!response.ok) {
          throw new Error(`HTTP Error: ${response.status} ${response.statusText}`);
        }

        if (!response.body) {
          throw new Error('Response body is empty');
        }

        options.onConnect?.();

        const parser = createParser({
          onEvent: (event) => options.onEvent(event),
          onError: (error) => options.onError?.(error),
        });

        this.#streamReader = createTextStreamReader(response.body, {
          onChunk: (chunk) => parser.feed(chunk),
          onComplete: () => this.disconnect(),
          onError: (error) => options.onError?.(error),
          signal: this.#abortController?.signal,
        });

        await this.#streamReader.read();
      } catch (error) {
        if (error instanceof Error && error.name === 'AbortError') {
          return; // Expected on disconnect
        }
        options.onError?.(error as Error);
        throw error; // Throw to allow retry
      }
    };

    try {
      if (options.enableAutoReconnect) {
        await withRetry(connectAndRead, {
          maxAttempts: options.maxRetryAttempts ?? 5,
          initialDelay: options.retryDelay ?? 1000,
          shouldRetry: (error) => error.message.includes('HTTP Error'),
        });
      } else {
        await connectAndRead();
      }
    } catch (error) {
      // Final error after retries
      options.onError?.(error as Error);
    } finally {
      this.disconnect();
    }
  }

  disconnect(): void {
    if (!this.#isConnected) {
      return;
    }
    this.#isConnected = false;
    this.#streamReader?.cancel();
    this.#abortController?.abort();
    this.#abortController = null;
    this.#streamReader = null;
    this.#callbacks.onDisconnect?.();
  }

  get connected(): boolean {
    return this.#isConnected;
  }
}
