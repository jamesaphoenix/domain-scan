// CJK identifiers in interfaces
export interface ユーザー {
    取得(): Promise<string>;
    更新(データ: string): Promise<void>;
}

export interface データベース {
    接続(): Promise<boolean>;
    切断(): Promise<void>;
}
