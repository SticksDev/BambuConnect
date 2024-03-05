export type Config = {
	is_first_run: boolean;
	bambu_info: BambuInfo;
	bambu_devices: Device[];
};

export type BambuInfo = {
	jwt: string;
	refresh_token: string;
	refresh_token_expires_at: number;
	jwt_expires_at: number;
	jwt_last_refresh: number;
};

export type BambuLoginResponse = {
	token: string;
	refresh_token: string;
};

export type BambuDevicesResponse = {
	message: string;
	code?: string;
	error?: string;
	devices: Device[];
};

export type BambuDiscoveryResponse = {
	devices: Device[];
}

export type Device = {
	dev_id: string;
	name: string;
	online: boolean;
	ip?: string;
	print_status: string;
	dev_model_name: string;
	dev_product_name: string;
	dev_access_code: string;
	nozzle_diameter: number;
};
