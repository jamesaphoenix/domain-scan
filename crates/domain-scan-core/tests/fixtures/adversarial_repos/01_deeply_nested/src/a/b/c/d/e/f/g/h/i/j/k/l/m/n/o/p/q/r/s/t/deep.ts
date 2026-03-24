// 20+ levels of nesting
export interface DeepInterface {
    getValue(): string;
    process(input: number): Promise<boolean>;
}

export class DeepClass implements DeepInterface {
    getValue(): string {
        return "deep";
    }

    process(input: number): Promise<boolean> {
        return Promise.resolve(input > 0);
    }
}
