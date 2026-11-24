import React, { useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { createClient } from '@supabase/supabase-js';
import { maxDiagnosticBytes, summarizeDiagnostic } from './diagnostic.mjs';
import './styles.css';

const url = import.meta.env.VITE_SUPABASE_URL;
const key = import.meta.env.VITE_SUPABASE_PUBLISHABLE_KEY;
const allowedConfig = typeof url === 'string' && /^https:\/\/[a-z0-9-]+\.supabase\.co\/?$/.test(url)
  && typeof key === 'string' && /^sb_publishable_[A-Za-z0-9_-]+$/.test(key) && !key.endsWith('REPLACE_ME');
const client = allowedConfig ? createClient(url, key, { auth: { flowType: 'pkce' } }) : null;
const platforms = { windows: 'Windows', linux: 'Linux', wsl2: 'WSL2' };
const dateTime = new Intl.DateTimeFormat('pt-BR', { dateStyle: 'short', timeStyle: 'short' });

function Diagnostics({ device }) {
  const [history, setHistory] = useState(null);
  const [preview, setPreview] = useState(null);
  const [notice, setNotice] = useState('');
  const [reading, setReading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [refresh, setRefresh] = useState(0);

  useEffect(() => {
    const abort = new AbortController();
    async function loadHistory() {
      try {
        const { data, error } = await client.from('central_diagnostics')
          .select('collected_at,memory_used,disk_used,checks_total,checks_review,checks_unavailable')
          .eq('device_id', device.id).order('collected_at', { ascending: false }).limit(20).abortSignal(abort.signal);
        if (error) throw error;
        if (!abort.signal.aborted) setHistory(data);
      } catch {
        if (!abort.signal.aborted) setNotice('Não foi possível consultar os diagnósticos.');
      }
    }
    loadHistory();
    return () => abort.abort();
  }, [device.id, refresh]);

  async function readReport(event) {
    const input = event.currentTarget;
    const file = input.files?.[0];
    input.value = '';
    setNotice('');
    setPreview(null);
    if (!file) return;
    if (file.size > maxDiagnosticBytes) { setNotice('O relatório excede o limite de 256 KiB.'); return; }
    setReading(true);
    try {
      setPreview(summarizeDiagnostic(JSON.parse(await file.text())));
    } catch {
      setNotice('Selecione um diagnóstico JSON válido exportado pela Central de diagnóstico.');
    } finally {
      setReading(false);
    }
  }

  async function importReport() {
    if (!preview) return;
    setSaving(true);
    setNotice('');
    try {
      const { error } = await client.from('central_diagnostics').insert({ device_id: device.id, ...preview });
      if (error?.code === '23505') {
        setNotice('Este horário já está registrado para o dispositivo.');
        return;
      }
      if (error) throw error;
      setPreview(null);
      setRefresh((value) => value + 1);
      setNotice('Resumo importado.');
    } catch {
      setNotice('Não foi possível importar o diagnóstico.');
    } finally {
      setSaving(false);
    }
  }

  return <section className="diagnostics">
    <h3>Diagnósticos de {device.label}</h3>
    <p>Escolha um arquivo JSON do aplicativo local. O envio inclui horário, indicadores e contagem de verificações.</p>
    <label>Arquivo de diagnóstico<input type="file" accept=".json,application/json" onChange={readReport} disabled={reading || saving} /></label>
    {preview && <section className="preview" aria-label="Prévia do diagnóstico">
      <DiagnosticSummary report={preview} />
      <button type="button" disabled={saving} onClick={importReport}>{saving ? 'Importando…' : 'Importar resumo'}</button>
    </section>}
    <p role="status">{notice}</p>
    {history?.length === 0 && <p>Nenhum diagnóstico importado.</p>}
    {history?.length > 0 && <section aria-label="Diagnósticos anteriores">{history.map((row) => <article key={row.collected_at}><DiagnosticSummary report={row} /></article>)}</section>}
  </section>;
}

function DiagnosticSummary({ report }) {
  return <div className="summary"><strong><time dateTime={report.collected_at}>{dateTime.format(new Date(report.collected_at))}</time></strong>
    <div><meter min="0" max="100" value={report.memory_used} aria-label="Memória ocupada" /> {report.memory_used}% de memória em uso</div>
    {report.disk_used !== null && <div><meter min="0" max="100" value={report.disk_used} aria-label="Disco ocupado" /> {report.disk_used}% de disco ocupado</div>}
    <span>{report.checks_total} verificações · {report.checks_review} para revisar · {report.checks_unavailable} indisponíveis</span>
  </div>;
}

function Account({ recovery, onRecoveryComplete }) {
  const [mode, setMode] = useState('login');
  const [status, setStatus] = useState('');
  const [pending, setPending] = useState(false);

  async function submit(event) {
    event.preventDefault();
    const form = event.currentTarget;
    const values = new FormData(form);
    const email = String(values.get('email') ?? '').trim();
    const password = String(values.get('password') ?? '');
    const redirectTo = window.location.origin;
    setPending(true);
    setStatus('');
    try {
      if (recovery) {
        const { error } = await client.auth.updateUser({ password });
        if (error) throw error;
        onRecoveryComplete();
      } else if (mode === 'register') {
        const { error } = await client.auth.signUp({ email, password, options: { emailRedirectTo: redirectTo } });
        if (error) throw error;
        setStatus('Confira sua caixa de entrada para continuar.');
      } else if (mode === 'reset') {
        const { error } = await client.auth.resetPasswordForEmail(email, { redirectTo });
        if (error) throw error;
        setStatus('Se houver uma conta, o endereço receberá instruções de recuperação.');
      } else {
        const { error } = await client.auth.signInWithPassword({ email, password });
        if (error) throw error;
      }
      form.reset();
    } catch {
      setStatus(mode === 'login' && !recovery ? 'Não foi possível entrar. Confira os dados e tente novamente.' : 'Não foi possível concluir a solicitação. Tente novamente.');
    } finally {
      setPending(false);
    }
  }

  const title = recovery ? 'Defina uma nova senha' : { login: 'Entrar na conta', register: 'Criar conta', reset: 'Recuperar acesso' }[mode];
  return <section className="panel">
    <h2>{title}</h2>
    <form onSubmit={submit}>
      {!recovery && <label>E-mail<input type="email" name="email" autoComplete="email" required maxLength={254} /></label>}
      {(recovery || mode !== 'reset') && <label>Senha<input type="password" name="password" autoComplete={!recovery && mode === 'login' ? 'current-password' : 'new-password'} required minLength={recovery || mode === 'register' ? 12 : 1} maxLength={72} /></label>}
      <button disabled={pending}>{pending ? 'Aguarde…' : title}</button>
    </form>
    {!recovery && <nav aria-label="Conta">{Object.entries({ login: 'Já tenho conta', register: 'Criar conta', reset: 'Esqueci a senha' }).map(([id, label]) => id !== mode && <button type="button" className="quiet" key={id} disabled={pending} onClick={() => { setMode(id); setStatus(''); }}>{label}</button>)}</nav>}
    <p role="status">{status}</p>
  </section>;
}

function Dashboard({ session }) {
  const [devices, setDevices] = useState(null);
  const [status, setStatus] = useState('');
  const [pending, setPending] = useState(false);
  const [refresh, setRefresh] = useState(0);
  const [selected, setSelected] = useState(null);
  const currentDevice = devices?.find((device) => device.id === selected);

  useEffect(() => {
    const abort = new AbortController();
    async function listDevices() {
      try {
        const { data, error } = await client.from('central_devices').select('id,label,platform,created_at')
          .order('created_at', { ascending: false }).limit(50).abortSignal(abort.signal);
        if (error) throw error;
        if (!abort.signal.aborted) setDevices(data);
      } catch {
        if (!abort.signal.aborted) setStatus('Não foi possível consultar os dispositivos.');
      }
    }
    listDevices();
    return () => abort.abort();
  }, [refresh]);

  async function addDevice(event) {
    event.preventDefault();
    const form = event.currentTarget;
    const values = new FormData(form);
    const label = String(values.get('label')).trim();
    const platform = String(values.get('platform'));
    if (!label || !Object.hasOwn(platforms, platform)) { setStatus('Informe um nome e selecione a plataforma.'); return; }
    setPending(true);
    setStatus('');
    try {
      const { error } = await client.from('central_devices').insert({ label, platform });
      if (error) throw error;
      form.reset();
      setStatus('Dispositivo registrado no inventário.');
      setRefresh((value) => value + 1);
    } catch {
      setStatus('Não foi possível registrar o dispositivo.');
    } finally {
      setPending(false);
    }
  }

  async function signOut() {
    try {
      const { error } = await client.auth.signOut({ scope: 'local' });
      if (error) throw error;
    } catch {
      setStatus('Não foi possível encerrar a sessão.');
    }
  }

  return <section className="panel">
    <div className="heading"><div><h2>Dispositivos</h2><span>{session.user.email}</span></div><button type="button" className="quiet" onClick={signOut}>Sair</button></div>
    <p>Registre um dispositivo para importar os diagnósticos que você exportou no aplicativo local.</p>
    <form className="device" onSubmit={addDevice}>
      <label>Nome do dispositivo<input name="label" maxLength={80} required /></label>
      <label>Plataforma<select name="platform">{Object.entries(platforms).map(([id, name]) => <option key={id} value={id}>{name}</option>)}</select></label>
      <button disabled={pending}>{pending ? 'Registrando…' : 'Registrar'}</button>
    </form>
    {devices?.length === 0 && <p>Nenhum dispositivo registrado.</p>}
    {devices?.length > 0 && <ul>{devices.map((device) => <li key={device.id}><div><strong>{device.label}</strong> · {platforms[device.platform]}</div><button type="button" className="quiet" aria-pressed={device.id === selected} onClick={() => setSelected(device.id)}>Ver diagnósticos</button></li>)}</ul>}
    <p role="status">{status}</p>
    {currentDevice && <Diagnostics key={currentDevice.id} device={currentDevice} />}
  </section>;
}

function App() {
  const [session, setSession] = useState(undefined);
  const [recovery, setRecovery] = useState(false);
  useEffect(() => {
    if (!client) return;
    const { data } = client.auth.onAuthStateChange((event, value) => {
      if (event === 'PASSWORD_RECOVERY') setRecovery(true);
      if (event === 'SIGNED_OUT') setRecovery(false);
      setSession(value);
    });
    return () => data.subscription.unsubscribe();
  }, []);

  return <main><header>Central de diagnóstico</header><section className="intro"><h1>Seus dispositivos em um só lugar</h1><p>Uma ferramenta gratuita para acompanhar o estado dos seus computadores e organizar os resultados das verificações.</p></section>
    {!client ? <section className="panel"><h2>Portal por configurar</h2><p>Configure o projeto Supabase antes de habilitar o acesso.</p></section>
      : session === undefined ? <p role="status">Verificando sessão…</p>
      : session && !recovery ? <Dashboard key={session.user.id} session={session} />
      : <Account recovery={recovery} onRecoveryComplete={() => setRecovery(false)} />}
  </main>;
}

createRoot(document.getElementById('root')).render(<App />);
