<script lang="ts">
	import { invoke } from '@tauri-apps/api/tauri';
	import { awaiter } from '$lib/utils';
	import { onMount } from 'svelte';
	import type { Config, Device } from '$lib/types';
	import { clipboard, dialog } from '@tauri-apps/api';

	let loadingDevices = true;
	let devices: Device[] = [];

	onMount(async () => {
		const [config, configError] = await awaiter(invoke('get_config') as Promise<Config>);

		if (configError || !config) {
			await dialog.message(
				`Something went wrong while getting the configuration. We've copied the error to your clipboard. Please report this issue on GitHub.\n\nError: ${configError ?? 'Config was null'}\n\nThe application will now close.`,
				{ title: 'BambuConnect | Configuration Error', type: 'error' }
			);

			await clipboard.writeText(configError);
			invoke('quit', { code: 1 });
			return;
		}

		// Make a fake array filled with 10 of the same device from the config
		devices = config.bambu_devices;
		loadingDevices = false;
	});
</script>

<div class="container bg-zinc-800 w-screen h-screen">
	<!-- Top header, Bambu Connect at the left side, settings icon on the right -->
	<div class="flex justify-between items-center h-16 px-4 bg-zinc-900">
		<div class="flex items-center">
			<img src="/images/logo.svg" alt="Bambu Connect" class="w-8 h-8" />
			<h1 class="text-2xl font-bold text-gray-200 ml-2">Bambu Connect</h1>
		</div>
		<svg
			xmlns="http://www.w3.org/2000/svg"
			fill="none"
			viewBox="0 0 24 24"
			stroke-width="1.5"
			stroke="currentColor"
			class="w-6 h-6 text-white"
		>
			<path
				stroke-linecap="round"
				stroke-linejoin="round"
				d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.325.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.431l-1.003.827c-.293.241-.438.613-.43.992a7.723 7.723 0 0 1 0 .255c-.008.378.137.75.43.991l1.004.827c.424.35.534.955.26 1.43l-1.298 2.247a1.125 1.125 0 0 1-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.47 6.47 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.281c-.09.543-.56.94-1.11.94h-2.594c-.55 0-1.019-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 0 1-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 0 1-1.369-.49l-1.297-2.247a1.125 1.125 0 0 1 .26-1.431l1.004-.827c.292-.24.437-.613.43-.991a6.932 6.932 0 0 1 0-.255c.007-.38-.138-.751-.43-.992l-1.004-.827a1.125 1.125 0 0 1-.26-1.43l1.297-2.247a1.125 1.125 0 0 1 1.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.086.22-.128.332-.183.582-.495.644-.869l.214-1.28Z"
			/>
			<path
				stroke-linecap="round"
				stroke-linejoin="round"
				d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z"
			/>
		</svg>
	</div>

	<!-- Devices container -->
	<div class="flex flex-col mt-4 px-3">
		<h2 class="text-2xl font-bold text-gray-200">Devices</h2>
		<p class="text-gray-400 mt-1">Manage your connected devices below</p>
		<div class="flex flex-wrap justify-center mt-2 max-h-[75vh] overflow-x-scroll rounded-lg">
			{#if loadingDevices}
				<div class="flex flex-col items-center">
					<div role="status" class="mb-2">
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

					<p class="text-gray-400 animate-pulse">Loading devices...</p>
				</div>
			{:else if devices.length === 0}
				<p class="text-gray-400">No devices found</p>
			{:else}
				{#each devices as device}
					<!-- Device card -->
					<div class="flex flex-col bg-zinc-700 rounded-lg p-4 m-2 w-full">
						<!-- Display name, status as a badge next to the name-->
						<div class="flex items-center">
							<h3 class="text-xl font-bold text-gray-200">{device.name}</h3>
							<!-- Green flashing dot if online, red if not -->
							<span
								class="inline-flex items-center bg-green-100 text-green-800 text-xs font-medium px-2.5 py-0.5 rounded-full dark:bg-green-900 dark:text-green-300 ml-2"
							>
								<span class="w-2 h-2 me-1 bg-green-500 rounded-full"></span>
								Online
							</span>
						</div>

						<!-- Device info. Show the IP address, product name, and the last time it was seen -->
						<div class="flex flex-col mt-2">
							<p class="text-gray-400">IP: {device.ip}</p>
							<p class="text-gray-400">Product: {device.dev_product_name}</p>
							<p class="text-gray-400">Last seen: --</p>
						</div>
					</div>
				{/each}
			{/if}
		</div>
	</div>
</div>
