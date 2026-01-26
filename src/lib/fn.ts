export type ThrottleFn<T extends never[]> = (...args: T) => PromiseLike<void>;

export const throttle = <T extends never[]>(
  fn: (...args: T) => PromiseLike<never>
): ThrottleFn<T> => {
  let isExecuting = false;
  let pendingArgs: T | undefined;

  const executeGuard = async (...args: T) => {
    isExecuting = true;

    // Remove pending call before executing
    pendingArgs = undefined;
    try {
      await fn(...args);
    } finally {
      isExecuting = false;
    }
  };

  return async (...args) => {
    if (!isExecuting) {
      let executeArgs: T | undefined = args;
      // Nothing is executing, immediately execute the function
      while (executeArgs) {
        await executeGuard(...executeArgs);
        // Execute the pending call, if exists.
        executeArgs = pendingArgs;
      }
      return;
    }

    // Something is currently executing, queue the args and set a timeout
    pendingArgs = args;
  };
};
export function debounce<T extends any[]>(
  fn: (...args: T) => any,
  wait: number,
  options: { maxWait?: number } = {}
) {
  let timeout: any;
  let maxTimeout: any;
  let lastArgs: T;
  let lastCallTime: number | null = null;

  const invoke = () => {
    clearTimeout(timeout);
    clearTimeout(maxTimeout);
    timeout = maxTimeout = null;
    lastCallTime = null;
    fn(...lastArgs);
  };

  const debounced = (...args: T) => {
    lastArgs = args;
    const now = Date.now();

    if (lastCallTime === null) {
      lastCallTime = now;
    }

    clearTimeout(timeout);
    timeout = setTimeout(invoke, wait);

    if (options.maxWait && !maxTimeout) {
      maxTimeout = setTimeout(invoke, options.maxWait);
    }
  };

  debounced.cancel = () => {
    clearTimeout(timeout);
    clearTimeout(maxTimeout);
    timeout = maxTimeout = null;
    lastCallTime = null;
  };

  debounced.flush = () => {
    if (timeout) {
      invoke();
    }
  };

  return debounced;
}
