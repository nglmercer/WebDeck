// Window globals installed by the app (no external script globals remain:
// jQuery was unused, Toastify is a bundled typed import, socket.io stays
// behind a guarded lookup in wireup.ts).

interface Window {
  folder?: (folderId: string) => void;
  deleteFolder?: (folderName: string) => void;
  togglePasswordVisibility?: (id: string, iconId: string) => void;
  send_data?: (message: string) => void;
  goFullscreenIFRAME?: () => void;
}
