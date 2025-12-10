export default class Deferred<T> {
    promise: Promise<T>;
    value: T | undefined;
    resolve: (value: T | PromiseLike<T>) => void = () => {
        /**/
    };
    reject: (reason?: any) => void = () => {
        /**/
    };

    constructor(value?: T) {
        this.value = value;
        this.promise = new Promise<T>((resolve, reject) => {
            this.reject = reject;
            this.resolve = resolve;
        });
    }
}
