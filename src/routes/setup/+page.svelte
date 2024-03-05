<script lang="ts">
	let loading = true;
	let errored = false;
	let step = 0;
	let status = 'Contacting Bambu...';

	let username = '';
	let password = '';

	let validationErrors = {
		username: '',
		password: ''
	};

	import { invoke } from '@tauri-apps/api/tauri';
	import { dialog, clipboard } from '@tauri-apps/api';
	import { awaiter } from '$lib/utils';
	import { onMount } from 'svelte';
	import type {
		BambuDevicesResponse,
		BambuDiscoveryResponse,
		BambuLoginResponse
	} from '$lib/types';

	onMount(async () => {
		const [config, configError] = await awaiter(invoke('get_config'));

		if (configError) {
			await dialog.message(
				`Something went wrong while getting the configuration. We've copied the error to your clipboard. Please report this issue on GitHub.\n\nError: ${configError}\n\nThe application will now close.`,
				{ title: 'BambuConnect | Configuration Error', type: 'error' }
			);

			await clipboard.writeText(configError);
			invoke('quit', { code: 1 });
			return;
		}

		loading = false;
	});

	async function loginToBambuAndFetchData() {
		const [loginResponseRaw, loginError] = await awaiter(
			invoke('login_to_bambu', { username, password }) as Promise<string>
		);

		if (loginError || !loginResponseRaw) {
			status = 'Failed to authenticate with Bambu';
			await dialog.message(
				`Something went wrong while authenticating with Bambu. Please ensure you entered the correct username and password. We've copied the error to your clipboard. Please report this issue on GitHub.\n\nError: ${loginError ?? 'Login response was null'}\n\n`,
				{ title: 'BambuConnect | Authentication Error', type: 'error' }
			);

			step = 1;
			return;
		}

		const loginResponse = JSON.parse(loginResponseRaw) as BambuLoginResponse;
		console.log(
			`[setup] got login response from rust. Response: ${JSON.stringify(loginResponse, null, 2)}`
		);

		status = 'Authenticated with Bambu. Fetching devices...';
		const [_, setJwtError] = await awaiter(invoke('set_jwt', { jwt: loginResponse.token }));
		const [devicesRaw, devicesError] = await awaiter(invoke('fetch_devices') as Promise<string>);

		if (setJwtError || devicesError || !devicesRaw) {
			status = 'Failed to fetch devices from Bambu';
			await dialog.message(
				`Something went wrong while fetching devices from Bambu. We've copied the error to your clipboard. Please report this issue on GitHub.\n\nError: ${setJwtError ?? devicesError ?? 'Unknown error occurred'}\n\n`,
				{ title: 'BambuConnect | Fetch Devices Error', type: 'error' }
			);

			step = 1;
			return;
		}

		const devices = JSON.parse(devicesRaw) as BambuDevicesResponse;
		console.log(`[setup] got devices from rust. Response: ${JSON.stringify(devices, null, 2)}`);

		status = `Found ${devices.devices.length} devices. Discovering... (This may take a while)`;
		const [discoveryRaw, discoveryError] = await awaiter(
			invoke('discover_devices', { devices: devices.devices }) as Promise<string>
		);

		if (discoveryError || !discoveryRaw || discoveryRaw === null) {
			status = 'Failed to discover devices';
			await dialog.message(
				`Something went wrong while discovering devices. We've copied the error to your clipboard. Please report this issue on GitHub.\n\nError: ${discoveryError ?? 'Discovery response was null'}\n\n`,
				{ title: 'BambuConnect | Discovery Error', type: 'error' }
			);

			step = 1;
			return;
		}

		status = 'Discovery complete. Saving devices...';
		const discovery = JSON.parse(discoveryRaw) as BambuDiscoveryResponse;

		// Construct the save object
		const saveObject = {
			is_first_run: false,
			bambu_info: {
				jwt: loginResponse.token,
				refresh_token: loginResponse.refresh_token
			},
			bambu_devices: discovery.devices
		};

		const [_unused, saveError] = await awaiter(invoke('save_config', { config: saveObject }));

		if (!saveError) {
			step = 3;
		} else {
			status = 'Failed to save devices';
			await dialog.message(
				`Something went wrong while saving devices. We've copied the error to your clipboard. Please report this issue on GitHub.\n\nError: ${saveError}\n\n`,
				{ title: 'BambuConnect | Save Devices Error', type: 'error' }
			);

			step = 1;
		}
	}

	function validateLoginForm() {
		if (!username) {
			validationErrors.username = 'Username is required';
		} else {
			validationErrors.username = '';
		}

		if (!password) {
			validationErrors.password = 'Password is required';
		} else {
			validationErrors.password = '';
		}

		if (username && password) {
			step = 2;
			loginToBambuAndFetchData();
		}
	}
</script>

<div class="container bg-zinc-800 w-screen h-screen">
	<!-- Centered loading spinner -->
	<div class="flex flex-col justify-center items-center h-screen">
		{#if loading}
			<div role="status">
				<svg
					aria-hidden="true"
					class="w-14 h-14 text-gray-200 animate-spin dark:text-gray-600 fill-blue-600"
					viewBox="0 0 100 101"
					fill="none"
					xmlns="http://www.w3.org/2000/svg"
				>
					<path
						d="M100 50.5908C100 78.2051 77.6142 100.591 50 100.591C22.3858 100.591 0 78.2051 0 50.5908C0 22.9766 22.3858 0.59082 50 0.59082C77.6142 0.59082 100 22.9766 100 50.5908ZM9.08144 50.5908C9.08144 73.1895 27.4013 91.5094 50 91.5094C72.5987 91.5094 90.9186 73.1895 90.9186 50.5908C90.9186 27.9921 72.5987 9.67226 50 9.67226C27.4013 9.67226 9.08144 27.9921 9.08144 50.5908Z"
						fill="currentColor"
					/>
					<path
						d="M93.9676 39.0409C96.393 38.4038 97.8624 35.9116 97.0079 33.5539C95.2932 28.8227 92.871 24.3692 89.8167 20.348C85.8452 15.1192 80.8826 10.7238 75.2124 7.41289C69.5422 4.10194 63.2754 1.94025 56.7698 1.05124C51.7666 0.367541 46.6976 0.446843 41.7345 1.27873C39.2613 1.69328 37.813 4.19778 38.4501 6.62326C39.0873 9.04874 41.5694 10.4717 44.0505 10.1071C47.8511 9.54855 51.7191 9.52689 55.5402 10.0491C60.8642 10.7766 65.9928 12.5457 70.6331 15.2552C75.2735 17.9648 79.3347 21.5619 82.5849 25.841C84.9175 28.9121 86.7997 32.2913 88.1811 35.8758C89.083 38.2158 91.5421 39.6781 93.9676 39.0409Z"
						fill="currentFill"
					/>
				</svg>
				<span class="sr-only">Loading...</span>
			</div>

			<h1 class="text-2xl font-bold mt-2 text-white">Setup is loading...</h1>
		{/if}

		{#if errored}
			<svg
				xmlns="http://www.w3.org/2000/svg"
				fill="none"
				viewBox="0 0 24 24"
				stroke-width="1.5"
				stroke="currentColor"
				class="w-12 h-12 text-red-500 dark:text-red-400"
			>
				<path
					stroke-linecap="round"
					stroke-linejoin="round"
					d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126ZM12 15.75h.007v.008H12v-.008Z"
				/>
			</svg>

			<h1 class="text-2xl font-bold mt-2 text-white">Oops!</h1>
			<p class="text-red-500 mt-2 max-w-[30em]">
				An error was encountered while initializing the setup, please try closing and reopening
				BambuConnect.
			</p>
		{/if}

		{#if !loading && !errored}
			{#if step === 0}
				<h1 class="text-2xl font-bold mt-2 text-white">Welcome to BambuConnect!</h1>
				<p class="text-gray-200 break-words max-w-[30em]">
					Because this is your first time running BambuConnect, we need to set up the connection to
					your Bambu account. Click the button below to get started.
				</p>

				{#if errored}
					<p class="text-red-500 mt-2">
						An error occurred while initializing the setup. Please try again.
					</p>
				{/if}

				{#if !loading}
					<button
						on:click={() => {
							step = 1;
						}}
						class="mt-4 bg-blue-600 text-white px-4 py-2 rounded-md hover:bg-blue-700 transition-all duration-200 ease-in-out"
					>
						Begin Setup
					</button>
				{/if}
			{:else if step === 1}
				<h1 class="text-2xl font-bold mt-2 text-white">Authenticate with Bambu</h1>
				<p class="text-gray-200 break-words max-w-[30em]">
					For BambuConnect to work, we need to authenticate with your Bambu account. Because Bambu
					doesn't have an official API, or any form of OAuth, we need to use your credentials to
					authenticate. Don't worry, we don't store your credentials, and we only use them to
					authenticate with Bambu. You can see the source code for BambuConnect on <a
						href="https://github.com/NotoriousPyro/bambu-connect"
						target="_blank"
						rel="noopener noreferrer"
						class="underline">GitHub</a
					>. <br /><br /> Please enter your Bambu username and password below then press the "Authenticate"
					button.
				</p>

				<label for="username" class="text-gray-200 mt-4">Username</label>
				<input
					type="text"
					id="username"
					class="w-full bg-zinc-700 text-gray-200 px-4 py-2 rounded-md mt-2 max-w-96"
					bind:value={username}
				/>

				{#if validationErrors.username}
					<p class="text-red-500 mt-2">{validationErrors.username}</p>
				{/if}

				<label for="password" class="text-gray-200 mt-4">Password</label>
				<input
					type="password"
					id="password"
					class="w-full bg-zinc-700 text-gray-200 px-4 py-2 rounded-md mt-2 max-w-96"
					bind:value={password}
				/>

				{#if validationErrors.password}
					<p class="text-red-500 mt-2">{validationErrors.password}</p>
				{/if}

				{#if errored}
					<p class="text-red-500 mt-2">
						An error occurred while authenticating with Bambu. Please try again.
					</p>
				{/if}

				{#if !loading}
					<button
						on:click={validateLoginForm}
						class="mt-4 bg-blue-600 text-white px-4 py-2 rounded-md hover:bg-blue-700 transition-all duration-200 ease-in-out"
					>
						Authenticate
					</button>
				{/if}
			{:else if step == 2}
				<div role="status">
					<svg
						aria-hidden="true"
						class="w-14 h-14 text-gray-200 animate-spin dark:text-gray-600 fill-blue-600"
						viewBox="0 0 100 101"
						fill="none"
						xmlns="http://www.w3.org/2000/svg"
					>
						<path
							d="M100 50.5908C100 78.2051 77.6142 100.591 50 100.591C22.3858 100.591 0 78.2051 0 50.5908C0 22.9766 22.3858 0.59082 50 0.59082C77.6142 0.59082 100 22.9766 100 50.5908ZM9.08144 50.5908C9.08144 73.1895 27.4013 91.5094 50 91.5094C72.5987 91.5094 90.9186 73.1895 90.9186 50.5908C90.9186 27.9921 72.5987 9.67226 50 9.67226C27.4013 9.67226 9.08144 27.9921 9.08144 50.5908Z"
							fill="currentColor"
						/>
						<path
							d="M93.9676 39.0409C96.393 38.4038 97.8624 35.9116 97.0079 33.5539C95.2932 28.8227 92.871 24.3692 89.8167 20.348C85.8452 15.1192 80.8826 10.7238 75.2124 7.41289C69.5422 4.10194 63.2754 1.94025 56.7698 1.05124C51.7666 0.367541 46.6976 0.446843 41.7345 1.27873C39.2613 1.69328 37.813 4.19778 38.4501 6.62326C39.0873 9.04874 41.5694 10.4717 44.0505 10.1071C47.8511 9.54855 51.7191 9.52689 55.5402 10.0491C60.8642 10.7766 65.9928 12.5457 70.6331 15.2552C75.2735 17.9648 79.3347 21.5619 82.5849 25.841C84.9175 28.9121 86.7997 32.2913 88.1811 35.8758C89.083 38.2158 91.5421 39.6781 93.9676 39.0409Z"
							fill="currentFill"
						/>
					</svg>
					<span class="sr-only">Loading...</span>
				</div>

				<h1 class="text-2xl font-bold mt-2 text-white">Authenticating with Bambu...</h1>
				<p class="text-gray-200 break-words max-w-[30em]">
					{status}
				</p>
			{:else if step === 3}
				<svg
					xmlns="http://www.w3.org/2000/svg"
					fill="none"
					viewBox="0 0 24 24"
					stroke-width="1.5"
					stroke="currentColor"
					class="w-12 h-12 text-green-500 dark:text-green-400"
				>
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						d="M9 12.75 11.25 15 15 9.75M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z"
					/>
				</svg>
				<h1 class="text-2xl font-bold mt-2 text-white">Setup Complete!</h1>
				<p class="text-gray-200 break-words max-w-[30em]">
					You're all set! BambuConnect is now connected to your Bambu account and is ready to use.
				</p>

				<button
					on:click={() => {
						window.location.href = '/home';
					}}
					class="mt-4 bg-blue-600 text-white px-4 py-2 rounded-md hover:bg-blue-700 transition-all duration-200 ease-in-out"
				>
					Finish
				</button>
			{/if}
		{/if}
	</div>
</div>
