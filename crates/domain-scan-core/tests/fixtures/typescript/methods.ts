// Test fixture: TypeScript methods (in classes)

class Calculator {
  add(a: number, b: number): number {
    return a + b;
  }

  async fetchRate(currency: string): Promise<number> {
    return 1.0;
  }

  static create(): Calculator {
    return new Calculator();
  }

  private validate(input: number): boolean {
    return input >= 0;
  }
}
