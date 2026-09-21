import { useEffect, useRef, useState } from 'react';
import { CloudClient, CloudError, CloudWorkspace, type Account, type CloudFile, type Device, type FileContent } from './index.ts';

export function useCloud(importCopy: (file: FileContent) => Promise<string>) {
  const workspace = useRef<CloudWorkspace | null>(null);
  const lock = useRef(false);
  const login = useRef<AbortController | null>(null);
  const [authorizing, setAuthorizing] = useState(false);
  const [pairingCode, setPairingCode] = useState<string | null>(null);
  useEffect(() => () => { login.current?.abort(); }, []);
  const [account, setAccount] = useState<Account | null>(null);
  const [files, setFiles] = useState<CloudFile[]>([]);
  const [devices, setDevices] = useState<Device[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const clear = () => { workspace.current = null; setAccount(null); setFiles([]); setDevices([]); };
  const perform = async <T,>(operation: () => Promise<T>): Promise<T | null> => {
    if (lock.current) return null;
    lock.current = true; setBusy(true); setError(null); setNotice(null);
    try { return await operation(); }
    catch (reason) {
      if (reason instanceof CloudError && reason.status === 401) clear();
      setError(reason instanceof Error ? reason.message : String(reason));
      return null;
    } finally { lock.current = false; setBusy(false); }
  };
  const refreshState = async (client: CloudClient) => {
    const [nextAccount, nextFiles, nextDevices] = await Promise.all([client.account(), client.files(), client.devices()]);
    setAccount(nextAccount); setFiles(nextFiles); setDevices(nextDevices);
  };
  return {
    account, files, devices, busy, error, notice, authorizing, pairingCode,
    cancelLogin: () => login.current?.abort(),
    connectGoogle: (url: string | undefined, deviceName: string, openBrowser: (url: string) => Promise<unknown>) => perform(async () => {
      if (!url) throw new Error('Cloud sign-in is not available in this build yet. You can keep writing locally.');
      const client = new CloudClient(url);
      const controller = new AbortController(); login.current = controller; setAuthorizing(true);
      let flow;
      let completed = false;
      try {
        flow = await client.startGoogle(deviceName, controller.signal);
        if (controller.signal.aborted) return;
        setPairingCode(flow.pairingCode);
        await openBrowser(flow.authorizationUrl);
        const next = await client.waitForGoogle(flow, controller.signal);
        if (controller.signal.aborted) { await client.logout(); return; }
        completed = true;
        login.current = null; setAuthorizing(false); setPairingCode(null);
        workspace.current = new CloudWorkspace(client); setAccount(next);
        await refreshState(client);
      } catch (reason) {
        if (controller.signal.aborted) { setNotice('Sign-in canceled.'); return; }
        throw reason;
      } finally {
        login.current = null; setAuthorizing(false); setPairingCode(null);
        if (flow && !completed) void client.cancelGoogle(flow).catch(() => {});
      }
    }),
    refresh: () => perform(async () => { if (workspace.current) await refreshState(workspace.current.client); }),
    upload: (localId: string, name: string, content: string) => perform(async () => {
      if (!workspace.current) return;
      await workspace.current.upload(localId, name, content);
      await refreshState(workspace.current.client);
      setNotice('Uploaded. Other devices can now open this version.');
    }),
    open: (id: string) => perform(async () => {
      if (!workspace.current) return;
      await workspace.current.open(id, importCopy);
      setNotice('Opened as a new local document. Your other documents are unchanged.');
    }),
    logout: () => perform(async () => {
      try { await workspace.current?.client.logout(); }
      finally { clear(); }
    }),
    revoke: (id: string) => perform(async () => {
      if (!workspace.current) return;
      await workspace.current.client.revoke(id);
      if (id === account?.device.id) clear(); else await refreshState(workspace.current.client);
    }),
    billing: (kind: 'checkout' | 'portal') => perform(async () => workspace.current?.client.billing(kind)),
  };
}
export type CloudState = ReturnType<typeof useCloud>;
