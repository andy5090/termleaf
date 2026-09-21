import { Linking, Platform, Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';
import type { CloudState } from '@termleaf/cloud/react';
import type { Palette } from './presentation';

export function CloudPanel({ cloud, colors, upload }: { cloud: CloudState; colors: Palette; upload: () => void }) {
  const url = process.env.EXPO_PUBLIC_TERMLEAF_CLOUD_URL;
  const labelStyle = [styles.text, { color: colors.fg }];
  const button = (label: string, action: () => void) => <Pressable key={label} accessibilityRole="button" accessibilityLabel={label} accessibilityState={{ disabled: cloud.busy }} disabled={cloud.busy} onPress={action} style={[styles.button, { borderColor: colors.dim, opacity: cloud.busy ? 0.5 : 1 }]}><Text style={labelStyle}>{label}</Text></Pressable>;
  return <ScrollView keyboardShouldPersistTaps="handled" contentContainerStyle={styles.content}>
    <Text accessibilityRole="header" style={labelStyle}>Termleaf Cloud</Text>
    {!cloud.account ? <>
      <Text style={labelStyle}>Your writing, on every device.</Text>
      <Text style={labelStyle}>Continue with your Google account to open and share documents. You can always keep writing offline.</Text>
      {cloud.authorizing ? <>
        <Text style={labelStyle}>Finish signing in with Google in your browser, then enter this connection code:</Text>
        {cloud.pairingCode && <Text selectable accessibilityLabel={`Connection code ${cloud.pairingCode}`} style={[styles.pairing, { color: colors.fg }]}>{cloud.pairingCode}</Text>}
        <Text style={labelStyle}>Return here after approving this device. Your documents will appear automatically.</Text>
        <Pressable accessibilityRole="button" accessibilityLabel="Cancel sign-in" onPress={cloud.cancelLogin} style={[styles.button, { borderColor: colors.dim }]}><Text style={labelStyle}>Cancel sign-in</Text></Pressable>
      </> : button('Continue with Google', () => { void cloud.connectGoogle(url, `Termleaf ${Platform.OS}`, link => Linking.openURL(link)); })}
    </> : <>
      <Text style={labelStyle}>{cloud.account.user.email}</Text>
      <Text style={labelStyle}>Subscription: {cloud.account.subscription.status}{'\n'}{(cloud.account.subscription.usedBytes / 1048576).toFixed(2)} / {(cloud.account.subscription.quotaBytes / 1048576).toFixed(0)} MiB</Text>
      <Text style={labelStyle}>Upload this document to share it. Open copy keeps your other local documents.</Text>
      {button('Upload current document', upload)}
      {button('Refresh', () => { void cloud.refresh(); })}
      <Text accessibilityRole="header" style={labelStyle}>Cloud files</Text>
      {cloud.files.length === 0 && <Text style={labelStyle}>No cloud files yet.</Text>}
      {cloud.files.map(file => <View key={file.id} style={styles.file}><Text style={labelStyle}>{file.name} · v{file.version}</Text>{button(`Open ${file.name}`, () => { void cloud.open(file.id); })}</View>)}
      <Text accessibilityRole="header" style={labelStyle}>Devices</Text>
      {cloud.devices.map(device => <View key={device.id}><Text style={labelStyle}>{device.name}{device.id === cloud.account?.device.id ? ' (this device)' : ''}</Text>{device.id !== cloud.account?.device.id && button(`Revoke ${device.name}`, () => { void cloud.revoke(device.id); })}</View>)}
      {button('Sign out', () => { void cloud.logout(); })}
    </>}
    {cloud.busy && <Text accessibilityLiveRegion="polite" style={labelStyle}>Connecting…</Text>}
    {cloud.notice && <Text accessibilityLiveRegion="polite" style={labelStyle}>{cloud.notice}</Text>}
    {cloud.error && <Text accessibilityRole="alert" style={labelStyle}>{cloud.error}</Text>}
  </ScrollView>;
}
const styles = StyleSheet.create({
  content: { padding: 18, gap: 12 },
  text: { fontFamily: Platform.OS === 'ios' ? 'Menlo' : 'monospace', fontSize: 12, lineHeight: 20 },
  pairing: { fontFamily: 'monospace', fontSize: 30, letterSpacing: 5, textAlign: 'center', paddingVertical: 12 },
  button: { minHeight: 44, borderWidth: 1, alignItems: 'center', justifyContent: 'center', padding: 8 },
  file: { gap: 5, paddingVertical: 6 },
});
