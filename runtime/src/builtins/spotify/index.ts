import { call } from "webdeck:host";
import type { Command } from "../../generated/contracts";
type SpotifyCommand = Extract<Command, { type: "spotify" }>;
function request(
  method: string,
  url: string,
  headers: Record<string, string>,
  body: string,
): any {
  const response = call("network.fetch", {
    method,
    url,
    headers,
    body,
    timeoutMs: 10000,
  });
  if (response.status < 200 || response.status >= 300 || response.truncated)
    throw new Error("Spotify request failed");
  return response.body ? JSON.parse(response.body) : null;
}
export function execute(command: SpotifyCommand): unknown {
  const secrets = call("secrets.integration", { id: "spotify" });
  let token = secrets.token;
  if (
    !token.access_token ||
    (token.expires_at_ms ?? Date.parse(token.expires_at || "")) <=
      Date.now() + 30000 ||
    (!token.expires_at_ms && !token.expires_at)
  ) {
    if (!token.refresh_token)
      throw new Error("Connect Spotify from local settings first");
    const refresh = request(
      "POST",
      "https://accounts.spotify.com/api/token",
      {
        Authorization: `Basic ${call("crypto.base64", { text: `${secrets.clientId}:${secrets.clientSecret}` })}`,
        "Content-Type": "application/x-www-form-urlencoded",
      },
      `grant_type=refresh_token&refresh_token=${encodeURIComponent(token.refresh_token)}`,
    );
    token = {
      ...token,
      ...refresh,
      refresh_token: refresh.refresh_token || token.refresh_token,
      expires_at_ms: Date.now() + refresh.expires_in * 1000,
      expires_at: new Date(
        Date.now() + refresh.expires_in * 1000,
      ).toISOString(),
    };
    call("secrets.saveSpotifyToken", { token });
  }
  function api(method: string, path: string, body?: unknown): any {
    return request(
      method,
      `https://api.spotify.com/v1/${path}`,
      {
        Authorization: `Bearer ${token.access_token}`,
        "Content-Type": "application/json",
      },
      body === undefined ? "" : JSON.stringify(body),
    );
  }
  const action = command.action;
  const needsTrack = !["volume", "play_song", "play_playlist"].includes(action);
  const current = needsTrack ? api("GET", "me/player/currently-playing") : null;
  const track = current?.item?.id;
  const uri = current?.item?.uri;
  if (needsTrack && (!track || !uri))
    throw new Error("This Spotify action needs a current track");
  if (action === "save_song") api("PUT", "me/tracks", { ids: [track] });
  else if (action === "save_album") {
    const id = current.item.album?.id;
    if (!id) throw new Error("No current album");
    api("PUT", "me/albums", { ids: [id] });
  } else if (action === "volume") {
    const playback = api("GET", "me/player");
    const volume =
      command.change.type === "set"
        ? command.change.percent
        : Math.max(
            0,
            Math.min(
              100,
              (playback?.device?.volume_percent || 0) + command.change.percent,
            ),
          );
    api("PUT", `me/player/volume?volume_percent=${volume}`);
  } else if (action === "play_song" || action === "play_playlist") {
    const kind = action === "play_song" ? "track" : "playlist";
    const found = api(
      "GET",
      `search?q=${encodeURIComponent(command.target)}&type=${kind}&limit=1`,
    );
    const foundUri = found[`${kind}s`]?.items?.[0]?.uri;
    if (!foundUri) throw new Error("No matching Spotify item");
    api(
      "PUT",
      "me/player/play",
      kind === "track" ? { uris: [foundUri] } : { context_uri: foundUri },
    );
  } else if (
    ["add_to_playlist", "remove_from_playlist", "toggle_playlist"].includes(
      action,
    )
  ) {
    const found = api(
      "GET",
      `search?q=${encodeURIComponent(command.target)}&type=playlist&limit=1`,
    );
    const id = found.playlists?.items?.[0]?.id;
    if (!id || !/^[A-Za-z0-9]+$/.test(id))
      throw new Error("No matching Spotify playlist");
    const path = `playlists/${id}/tracks`;
    const remove =
      action === "toggle_playlist"
        ? api("GET", `${path}?limit=100`).items.some(
            (item: any) => item.track?.id === track,
          )
        : action === "remove_from_playlist";
    if (remove) api("DELETE", path, { tracks: [{ uri }] });
    else api("POST", path, { uris: [uri] });
  } else if (
    ["follow_artist", "unfollow_artist", "toggle_artist"].includes(action)
  ) {
    const artist = current.item.artists?.[0]?.id;
    if (!artist) throw new Error("No current artist");
    const follow =
      action === "toggle_artist"
        ? !api(
            "GET",
            `me/following/contains?type=artist&ids=${encodeURIComponent(artist)}`,
          )[0]
        : action === "follow_artist";
    api(follow ? "PUT" : "DELETE", "me/following?type=artist", {
      ids: [artist],
    });
  } else throw new Error("Invalid Spotify action");
  return {};
}
