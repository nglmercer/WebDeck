// Globals from CDN scripts (loaded in index.html exactly like upstream).
// Typed `any`: these come from untracked CDN bundles.

declare const Toastify: any;
declare const $: any;
declare const jQuery: any;

// socket.io client (conditionally loaded 1:1; 404s upstream too).
declare const io: any;

interface Window {
  folder?: (folderId: string) => void;
  deleteFolder?: (folderName: string) => void;
  togglePasswordVisibility?: (id: string, iconId: string) => void;
  send_data?: (message: string) => void;
  goFullscreenIFRAME?: () => void;
}
