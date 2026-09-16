/** Keeps stateful editor commands in issue order, even after a rejected command. */
export class IpcQueue {
  private tail: Promise<void> = Promise.resolve();

  run<T>(command: () => Promise<T>): Promise<T> {
    const result = this.tail.then(command, command);
    this.tail = result.then(
      () => undefined,
      () => undefined,
    );
    return result;
  }
}
