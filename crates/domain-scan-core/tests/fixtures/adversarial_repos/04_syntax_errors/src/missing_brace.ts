// Deliberately broken: unclosed interface
export interface BrokenInterface {
    findById(id: string): Promise<any>;
    findAll(): Promise<any[]>;
// Missing closing brace
