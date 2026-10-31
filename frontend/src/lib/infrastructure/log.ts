/**
 * The frontend's only logger. Everything that needs to report something calls `log`, and this is
 * the only file that touches `console` (the ESLint rule `no-console` is off here and nowhere else).
 *
 * The methods are arrow functions so they can be passed as callbacks, for example
 * `promise.catch(log.error)`.
 */
export const log = {
  /** Developer-oriented detail, hidden unless the browser console shows verbose output. */
  debug: (...args: unknown[]): void => {
    console.debug(...args);
  },
  /** Something worth knowing that is not a problem. */
  info: (...args: unknown[]): void => {
    console.info(...args);
  },
  /** Something unexpected that the interface recovered from. */
  warn: (...args: unknown[]): void => {
    console.warn(...args);
  },
  /** Something failed. */
  error: (...args: unknown[]): void => {
    console.error(...args);
  },
};
