import { useState } from 'react';
import type { CloudState } from '@termleaf/cloud/react';
import { native } from './native.ts';

export function CloudPanel({ cloud, upload }: { cloud: CloudState; upload: () => void }) {
  const url = import.meta.env.VITE_TERMLEAF_CLOUD_URL ?? (import.meta.env.DEV ? 'http://localhost:8787' : undefined);
  const [linkError, setLinkError] = useState<string | null>(null);
  const billing = async (kind: 'checkout' | 'portal') => {
    setLinkError(null);
    const link = await cloud.billing(kind);
    if (link) await native.openBilling(link).catch(error => setLinkError(String(error)));
  };
  return <div className="cloud-panel" aria-busy={cloud.busy}>
    {!cloud.account ? <div>
      <p>Your writing, on every device.</p>
      <p>Continue with your Google account to open and share your documents. You can always keep writing offline.</p>
      {cloud.authorizing ? <>
        <p>Finish signing in with Google in your browser, then enter this connection code:</p>
        {cloud.pairingCode && <p className="cloud-pairing" aria-label={`Connection code ${cloud.pairingCode}`}>{cloud.pairingCode}</p>}
        <p>Return here after approving this device. Your documents will appear automatically.</p>
        <button type="button" onClick={cloud.cancelLogin}>Cancel sign-in</button>
      </> : <button type="button" disabled={cloud.busy} onClick={() => void cloud.connectGoogle(url, 'Termleaf desktop', native.openAuthentication)}>Continue with Google</button>}
    </div> : <>
      <strong>{cloud.account.user.email}</strong>
      <p>Subscription: {cloud.account.subscription.status} · {(cloud.account.subscription.usedBytes / 1048576).toFixed(2)} / {(cloud.account.subscription.quotaBytes / 1048576).toFixed(0)} MiB</p>
      <p>Upload this document to share it. Opening a cloud file creates a new local document; updates use version checks.</p>
      <div className="cloud-actions">
        <button type="button" disabled={cloud.busy} onClick={upload}>Upload current document</button>
        <button type="button" disabled={cloud.busy} onClick={() => void cloud.refresh()}>Refresh</button>
        <button type="button" disabled={cloud.busy} onClick={() => void billing('checkout')}>Subscribe with Polar</button>
        <button type="button" disabled={cloud.busy} onClick={() => void billing('portal')}>Manage subscription</button>
      </div>
      <h3>Cloud files</h3>
      {cloud.files.length === 0 && <p>No cloud files yet.</p>}
      <ul className="cloud-files">{cloud.files.map(file => <li key={file.id}><span title={file.name}>{file.name} <small>v{file.version}</small></span><button type="button" disabled={cloud.busy} onClick={() => void cloud.open(file.id)}>Open copy</button></li>)}</ul>
      <h3>Devices</h3>
      <ul className="cloud-files">{cloud.devices.map(device => <li key={device.id}><span>{device.name}{device.id === cloud.account?.device.id ? ' (this device)' : ''}</span>{device.id !== cloud.account?.device.id && <button type="button" disabled={cloud.busy} onClick={() => void cloud.revoke(device.id)}>Revoke access</button>}</li>)}</ul>
      <button type="button" disabled={cloud.busy} onClick={() => void cloud.logout()}>Sign out</button>
    </>}
    {cloud.busy && <p role="status">Connecting…</p>}
    {cloud.notice && <p role="status">{cloud.notice}</p>}
    {(cloud.error || linkError) && <p role="alert">{cloud.error ?? linkError}</p>}
  </div>;
}
