declare module 'node:stream' {
  export class Writable {
    constructor(options?: unknown);
    write(chunk: unknown, encoding?: unknown, callback?: unknown): boolean;
    end(chunk?: unknown, encoding?: unknown, callback?: unknown): unknown;
    pipe<T>(destination: T): T;
    on(event: string, handler: (...args: unknown[]) => void): this;
    listenerCount(eventName: string): number;
    emit(eventName: string, ...args: unknown[]): boolean;
  }

  export class Transform extends Writable {
    constructor(options?: unknown);
  }
}
