export type Config = {
	is_first_run: boolean;
	bambu_info: BambuInfo;
};

export type BambuInfo = {
	jwt: string;
	refresh_token: string;
	refresh_token_expires_at: number;
	jwt_expires_at: number;
	jwt_last_refresh: number;
};
