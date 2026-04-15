export class CryptonError extends Error {
  public readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = 'CryptonError';
    this.code = code;
    // Restores correct prototype chain when transpiled to ES5
    Object.setPrototypeOf(this, new.target.prototype);
  }

  /** Type-safe narrowing without needing to import the class */
  static is(err: unknown): err is CryptonError {
    return err instanceof CryptonError;
  }
}
