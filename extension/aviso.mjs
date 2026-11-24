document.querySelector('#fechar').addEventListener('click', async () => {
  try {
    const tab = await chrome.tabs.getCurrent();
    if (tab?.id === undefined) throw new Error();
    await chrome.tabs.remove(tab.id);
  } catch {
    document.querySelector('#status').textContent = 'Feche a guia pelo botão do navegador.';
  }
});

document.querySelector('#som').addEventListener('click', async () => {
  let context;
  try {
    context = new AudioContext();
    await context.resume();
    const tone = context.createOscillator();
    const volume = context.createGain();
    const now = context.currentTime;
    tone.frequency.value = 520;
    volume.gain.setValueAtTime(0.12, now);
    volume.gain.exponentialRampToValueAtTime(0.001, now + 0.25);
    tone.connect(volume).connect(context.destination);
    tone.addEventListener('ended', () => context.close().catch(() => {}), { once: true });
    tone.start(now);
    tone.stop(now + 0.25);
  } catch {
    if (context) await context.close().catch(() => {});
    document.querySelector('#status').textContent = 'O som está indisponível. O aviso continua na tela.';
  }
});
