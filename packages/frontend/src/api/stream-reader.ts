// Stream reader utilities for handling ReadableStream operations
export interface StreamReaderOptions {
  onChunk?: (chunk: string) => void;
  onError?: (error: Error) => void;
  onComplete?: () => void;
  signal?: AbortSignal;
}

export interface StreamReader {
  read(): Promise<void>;
  cancel(): void;
}

export class TextStreamReader implements StreamReader {
  #reader: ReadableStreamDefaultReader<Uint8Array> | null = null;

  #decoder = new TextDecoder();

  #isReading = false;

  #stream: ReadableStream<Uint8Array>;

  #options: StreamReaderOptions;

  constructor(stream: ReadableStream<Uint8Array>, options: StreamReaderOptions = {}) {
    this.#stream = stream;
    this.#options = options;
  }

  async read(): Promise<void> {
    if (this.#isReading) {
      throw new Error('Stream is already being read');
    }

    this.#reader = this.#stream.getReader();
    this.#isReading = true;

    try {
      while (this.#isReading && !this.#options.signal?.aborted) {
        // eslint-disable-next-line no-await-in-loop
        const { done, value } = await this.#reader.read();

        if (done) {
          this.#options.onComplete?.();
          break;
        }

        const chunk = this.#decoder.decode(value, { stream: true });
        this.#options.onChunk?.(chunk);
      }
    } catch (error) {
      if (this.#isReading && !this.#options.signal?.aborted) {
        this.#options.onError?.(error as Error);
      }
    } finally {
      this.#cleanup();
    }
  }

  cancel(): void {
    this.#isReading = false;
    this.#cleanup();
  }

  #cleanup(): void {
    if (this.#reader) {
      this.#reader.releaseLock();
      this.#reader = null;
    }
    this.#isReading = false;
  }
}

// Utility functions for common stream operations
export function createTextStreamReader(
  stream: ReadableStream<Uint8Array>,
  options: StreamReaderOptions = {},
): StreamReader {
  return new TextStreamReader(stream, options);
}

export function readStreamToString(stream: ReadableStream<Uint8Array>): Promise<string> {
  return new Promise((resolve, reject) => {
    let result = '';

    const reader = createTextStreamReader(stream, {
      onChunk: (chunk) => {
        result += chunk;
      },
      onComplete: () => {
        resolve(result);
      },
      onError: reject,
    });

    reader.read().catch(reject);
  });
}

export function readStreamWithCallback(
  stream: ReadableStream<Uint8Array>,
  onData: (chunk: string) => void,
  options: Omit<StreamReaderOptions, 'onChunk'> = {},
): Promise<void> {
  return new Promise((resolve, reject) => {
    const reader = createTextStreamReader(stream, {
      ...options,
      onChunk: onData,
      onComplete: () => {
        options.onComplete?.();
        resolve();
      },
      onError: (error) => {
        options.onError?.(error);
        reject(error);
      },
    });

    reader.read().catch(reject);
  });
}
