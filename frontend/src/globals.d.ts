// Window globals installed by the app (no external script globals remain:
// jQuery was unused, Toastify and socket.io-client are bundled typed
// imports).

interface Window {
  folder?: (folderId: string) => void;
  deleteFolder?: (folderName: string) => void;
  togglePasswordVisibility?: (id: string, iconId: string) => void;
  send_data?: (message: string) => void;
  goFullscreenIFRAME?: () => void;
}
