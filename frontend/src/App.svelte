<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { api, ApiFailure, type ChecklistItem, type Cue, type ObsStatus, type PlatformLink, type Settings } from './api';
  import { formatDuration, isEditableTarget, makeId, timerMilliseconds, type TimerState } from './utils';

  const path = window.location.pathname.replace(/\/$/, '') || '/';
  const staticPage = path === '/privacy' ? 'privacy' : path === '/terms' ? 'terms' : null;
  let loading = true;
  let serviceError = '';
  let actionError = '';
  let announcement = '';
  let online = navigator.onLine;
  let settings: Settings = { obs_host: '127.0.0.1', obs_port: 4455, configured: false, password_saved: false };
  let checklist: ChecklistItem[] = [];
  let cues: Cue[] = [];
  let links: PlatformLink[] = [];
  let obs: ObsStatus = { connected: false, message: 'OBS connection not checked.', scenes: [], current_scene: null };
  let obsChecking = false;
  let saving = false;
  let now = Date.now();
  let timer: TimerState = { elapsed: 0, startedAt: null, running: false };
  let timerText = '00:00:00';

  let settingsDialog: HTMLDialogElement;
  let checklistDialog: HTMLDialogElement;
  let cuesDialog: HTMLDialogElement;
  let linksDialog: HTMLDialogElement;
  let shortcutsDialog: HTMLDialogElement;
  let resetDialog: HTMLDialogElement;
  let settingsHost = '127.0.0.1';
  let settingsPort = 4455;
  let settingsPassword = '';
  let clearPassword = false;
  let settingsError = '';
  let editorError = '';
  let editChecklist: ChecklistItem[] = [];
  let editCues: Cue[] = [];
  let editLinks: PlatformLink[] = [];
  let firstIncomplete: HTMLInputElement | null = null;

  $: timerText = formatDuration(timerMilliseconds(timer, now));
  $: completedCount = checklist.filter((item) => item.done).length;
  $: remainingCount = checklist.length - completedCount;

  function say(message: string, speak = false) {
    announcement = '';
    requestAnimationFrame(() => { announcement = message; });
    if (speak && 'speechSynthesis' in window) {
      speechSynthesis.cancel();
      speechSynthesis.speak(new SpeechSynthesisUtterance(message));
    }
  }

  function errorMessage(error: unknown): string {
    return error instanceof Error ? error.message : 'Something went wrong. Try again.';
  }

  async function loadAll() {
    loading = true;
    serviceError = '';
    const results = await Promise.allSettled([api.settings(), api.checklist(), api.cues(), api.links()]);
    const failed = results.find((result) => result.status === 'rejected');
    if (failed?.status === 'rejected') serviceError = errorMessage(failed.reason);
    if (results[0].status === 'fulfilled') settings = results[0].value;
    if (results[1].status === 'fulfilled') checklist = results[1].value;
    if (results[2].status === 'fulfilled') cues = results[2].value;
    if (results[3].status === 'fulfilled') links = results[3].value;
    loading = false;
    if (settings.configured) refreshObs(false);
  }

  async function refreshObs(announceResult = true) {
    obsChecking = true;
    actionError = '';
    try {
      obs = await api.obsStatus();
      if (announceResult) say(`${obs.message} Current scene: ${obs.current_scene ?? 'unknown'}.`);
    } catch (error) {
      obs = { connected: false, message: errorMessage(error), scenes: [], current_scene: null };
      if (announceResult) say(obs.message);
    } finally {
      obsChecking = false;
    }
  }

  async function triggerCue(cue: Cue) {
    actionError = '';
    say(`Changing scene to ${cue.scene_name}.`);
    try {
      obs = await api.setScene(cue.scene_name);
      say(`${cue.label}: scene changed to ${cue.scene_name}.`, true);
    } catch (error) {
      actionError = errorMessage(error);
      say(`Scene change failed. ${actionError}`);
    }
  }

  async function toggleItem(item: ChecklistItem) {
    const previous = checklist;
    checklist = checklist.map((candidate) => candidate.id === item.id ? { ...candidate, done: !candidate.done } : candidate);
    const changed = checklist.find((candidate) => candidate.id === item.id)!;
    say(`${changed.text}, marked ${changed.done ? 'complete' : 'not complete'}.`);
    try {
      checklist = await api.saveChecklist(checklist);
    } catch (error) {
      checklist = previous;
      actionError = `${errorMessage(error)} Your checklist change was not saved.`;
      say(actionError);
    }
  }

  function speakStatus() {
    const next = checklist.find((item) => !item.done);
    const scene = obs.connected ? `Current OBS scene is ${obs.current_scene}.` : 'OBS is not connected.';
    const task = next ? `Next checklist item: ${next.text}.` : 'Checklist complete.';
    say(`${scene} Timer ${timerText}. ${task}`, true);
  }

  function speakItem(item: ChecklistItem) {
    say(`${item.text}. ${item.done ? 'Complete.' : 'Not complete.'}`, true);
  }

  function toggleTimer() {
    const timestamp = Date.now();
    if (timer.running) {
      timer = { elapsed: timerMilliseconds(timer, timestamp), startedAt: null, running: false };
      say(`Timer paused at ${formatDuration(timer.elapsed)}.`);
    } else {
      timer = { ...timer, startedAt: timestamp, running: true };
      say(`Timer started at ${formatDuration(timer.elapsed)}.`);
    }
    persistTimer();
  }

  function resetTimer() {
    timer = { elapsed: 0, startedAt: null, running: false };
    persistTimer();
    resetDialog.close();
    say('Timer reset to zero.');
  }

  function persistTimer() {
    localStorage.setItem('stream-access-cues.timer', JSON.stringify(timer));
  }

  async function focusNextItem() {
    await tick();
    firstIncomplete?.focus();
    if (!firstIncomplete) say('Checklist complete. There is no incomplete item.');
  }

  function openSettings() {
    settingsHost = settings.obs_host;
    settingsPort = settings.obs_port;
    settingsPassword = '';
    clearPassword = false;
    settingsError = '';
    settingsDialog.showModal();
  }

  async function saveSettings() {
    saving = true;
    settingsError = '';
    try {
      settings = await api.saveSettings({
        obs_host: settingsHost,
        obs_port: Number(settingsPort),
        ...(settingsPassword || clearPassword ? { obs_password: clearPassword ? '' : settingsPassword } : {})
      });
      settingsDialog.close();
      say('OBS connection settings saved locally. Testing the connection.');
      await refreshObs();
    } catch (error) {
      settingsError = errorMessage(error);
    } finally { saving = false; }
  }

  function openChecklistEditor() {
    editChecklist = checklist.map((item) => ({ ...item }));
    editorError = '';
    checklistDialog.showModal();
  }

  async function saveChecklistEditor() {
    saving = true;
    actionError = '';
    try {
      checklist = await api.saveChecklist(editChecklist);
      checklistDialog.close();
      say(`Checklist saved with ${checklist.length} items.`);
    } catch (error) { editorError = errorMessage(error); }
    finally { saving = false; }
  }

  function openCuesEditor() {
    editCues = cues.map((cue) => ({ ...cue }));
    editorError = '';
    cuesDialog.showModal();
  }

  async function saveCuesEditor() {
    saving = true;
    actionError = '';
    try {
      cues = await api.saveCues(editCues);
      cuesDialog.close();
      say(`${cues.length} scene cues saved.`);
    } catch (error) { editorError = errorMessage(error); }
    finally { saving = false; }
  }

  function openLinksEditor() {
    editLinks = links.map((link) => ({ ...link }));
    editorError = '';
    linksDialog.showModal();
  }

  async function saveLinksEditor() {
    saving = true;
    actionError = '';
    try {
      links = await api.saveLinks(editLinks);
      linksDialog.close();
      say(`${links.length} metadata links saved.`);
    } catch (error) { editorError = errorMessage(error); }
    finally { saving = false; }
  }

  function keyboard(event: KeyboardEvent) {
    if (event.key === '?' && !isEditableTarget(event.target)) {
      event.preventDefault();
      shortcutsDialog.showModal();
      return;
    }
    if (!(event.ctrlKey || event.metaKey) || !event.shiftKey || isEditableTarget(event.target)) return;
    const key = event.key.toLowerCase();
    if (/^[1-9]$/.test(key)) {
      const cue = cues[Number(key) - 1];
      if (cue) { event.preventDefault(); triggerCue(cue); }
    } else if (key === 't') { event.preventDefault(); toggleTimer(); }
    else if (key === 'r') { event.preventDefault(); resetDialog.showModal(); }
    else if (key === 'c') { event.preventDefault(); focusNextItem(); }
    else if (key === 's') { event.preventDefault(); speakStatus(); }
  }

  onMount(() => {
    if (!staticPage) loadAll(); else loading = false;
    try {
      const stored = localStorage.getItem('stream-access-cues.timer');
      if (stored) timer = JSON.parse(stored) as TimerState;
    } catch { localStorage.removeItem('stream-access-cues.timer'); }
    const interval = window.setInterval(() => { now = Date.now(); }, 500);
    const onOnline = () => { online = true; say('Browser is online.'); };
    const onOffline = () => { online = false; say('Browser is offline. Local controls remain available; external metadata pages will not open.'); };
    window.addEventListener('online', onOnline);
    window.addEventListener('offline', onOffline);
    window.addEventListener('keydown', keyboard);
    return () => {
      clearInterval(interval);
      window.removeEventListener('online', onOnline);
      window.removeEventListener('offline', onOffline);
      window.removeEventListener('keydown', keyboard);
    };
  });
</script>

<a class="skip-link" href="#main">Skip to main controls</a>

<header class="site-header">
  <a class="wordmark" href="/" aria-label="Stream Access Cues home">
    <span class="brand-lamp" aria-hidden="true"></span>
    <span>Stream Access Cues</span>
  </a>
  {#if !staticPage}
    <nav aria-label="Utility navigation">
      <button class="quiet-button" type="button" onclick={() => shortcutsDialog.showModal()} aria-keyshortcuts="?">Shortcuts <kbd>?</kbd></button>
      <button class="quiet-button" type="button" onclick={openSettings}>Connection</button>
    </nav>
  {:else}
    <nav aria-label="Utility navigation"><a href="/">Return to cue surface</a></nav>
  {/if}
</header>

{#if !online}
  <div class="offline-banner" role="status"><strong>Browser offline.</strong> Saved controls still work; metadata pages need an internet connection.</div>
{/if}

<main id="main">
  {#if staticPage === 'privacy'}
    <article class="legal-page">
      <p class="eyebrow">Plain-language policy</p>
      <h1>Privacy</h1>
      <p class="lede">Stream Access Cues is local-first. It has no accounts, analytics, advertising, tracking pixels, or third-party scripts.</p>
      <h2>What is stored</h2>
      <p>The local service stores your OBS host, port and WebSocket password, checklist, scene cue names, and metadata links in its SQLite data directory. The browser stores only the session timer and an offline copy of the app shell.</p>
      <h2>Where it goes</h2>
      <p>Your saved data is not sent to Sociobot or any hosted service. OBS credentials are used only by your running local service to connect to the OBS host you configure. Opening a metadata link takes you to that platform under its privacy policy.</p>
      <h2>Remove your data</h2>
      <p>Edit or remove checklist, cue, and link entries in the app. To remove all data, stop the service and delete its configured <code>DATA_DIR</code>. Clear site data in your browser to remove the timer and offline shell.</p>
      <p>Effective 27 August 2026.</p>
    </article>
  {:else if staticPage === 'terms'}
    <article class="legal-page">
      <p class="eyebrow">Use terms</p>
      <h1>Terms</h1>
      <p class="lede">Stream Access Cues is free, open-source assistive software provided under the MIT License.</p>
      <h2>Your responsibility</h2>
      <p>Test your cues before going live. You control your OBS instance and external platform accounts. This tool launches platform pages but does not submit or guarantee metadata changes.</p>
      <h2>No affiliation</h2>
      <p>OBS, Twitch, and YouTube are referenced only to describe compatibility. This project is not endorsed by those projects or companies.</p>
      <h2>Warranty</h2>
      <p>The software is provided “as is,” without warranty, as detailed in the repository’s MIT License.</p>
      <p>Effective 27 August 2026.</p>
    </article>
  {:else}
    <section class="intro" aria-labelledby="page-title">
      <div>
        <p class="eyebrow">Local broadcast control · no account</p>
        <h1 id="page-title">Your stream, under your fingers.</h1>
        <p class="lede">Set the next task, change an OBS scene, and keep time—without wrestling an embedded web dock.</p>
      </div>
      <div class="system-state" aria-label="System status">
        <span class:lamp-good={obs.connected} class:lamp-warn={!obs.connected} class="status-lamp" aria-hidden="true"></span>
        <span><strong>{obs.connected ? 'OBS ready' : settings.configured ? 'OBS unavailable' : 'Setup needed'}</strong><small>{obs.connected ? `Scene: ${obs.current_scene ?? 'unknown'}` : obs.message}</small></span>
        <button type="button" class="text-button" onclick={() => refreshObs()} disabled={obsChecking || !settings.configured}>{obsChecking ? 'Checking…' : 'Check connection'}</button>
      </div>
    </section>

    <div class="live-region" aria-live="polite" aria-atomic="true">{announcement}</div>

    {#if serviceError}
      <section class="error-strip" role="alert">
        <div><strong>Local service unavailable</strong><p>{serviceError}</p></div>
        <button type="button" onclick={loadAll}>Retry connection</button>
      </section>
    {/if}
    {#if actionError}
      <section class="error-strip compact" role="alert">
        <p>{actionError}</p><button type="button" class="quiet-button" onclick={() => actionError = ''}>Dismiss</button>
      </section>
    {/if}

    {#if loading}
      <section class="loading-panel" aria-busy="true" aria-label="Loading saved cue surface">
        <span class="meter" aria-hidden="true"></span><p>Warming up the local control surface…</p>
      </section>
    {:else}
      {#if !settings.configured}
        <section class="onboarding" aria-labelledby="onboarding-title">
          <div class="onboarding-copy">
            <p class="panel-number">Start here · 01</p>
            <h2 id="onboarding-title">Connect the controls you already use.</h2>
            <p>Enable the WebSocket server in OBS under <strong>Tools → WebSocket Server Settings</strong>. Then save the local host, port, and optional password here.</p>
            <button class="primary-button" type="button" onclick={openSettings}>Configure OBS connection</button>
            <p class="privacy-note">Your password stays in this service’s local SQLite file. It is never sent to our servers.</p>
          </div>
          <picture>
            <source media="(max-width: 700px)" srcset="/assets/control-panel-hero-768.webp" />
            <img src="/assets/control-panel-hero-1280.webp" width="1280" height="853" alt="Illustrated vintage broadcast console with tactile switches, three amber scene keys, a green status lamp, timer dial, and blank cue card" fetchpriority="high" decoding="async" />
          </picture>
        </section>
      {/if}

      <section class="timer-panel panel" aria-labelledby="timer-heading">
        <div class="panel-heading">
          <div><p class="panel-number">Clock · A</p><h2 id="timer-heading">Session timer</h2></div>
          <button type="button" class="quiet-button" onclick={speakStatus}>Speak status</button>
        </div>
        <div class="timer-row">
          <output class="timer-readout" aria-label={`Elapsed time ${timerText}`}>{timerText}</output>
          <div class="timer-actions">
            <button class="primary-button" type="button" onclick={toggleTimer} aria-keyshortcuts="Control+Shift+T Meta+Shift+T">{timer.running ? 'Pause timer' : timer.elapsed ? 'Resume timer' : 'Start timer'}</button>
            <button type="button" onclick={() => resetDialog.showModal()} disabled={!timer.running && timer.elapsed === 0} aria-keyshortcuts="Control+Shift+R Meta+Shift+R">Reset</button>
          </div>
        </div>
      </section>

      <div class="deck-grid">
        <section class="panel checklist-panel" aria-labelledby="checklist-heading">
          <div class="panel-heading">
            <div><p class="panel-number">Preflight · B</p><h2 id="checklist-heading">Spoken checklist</h2></div>
            <button type="button" class="quiet-button" onclick={openChecklistEditor}>Edit</button>
          </div>
          {#if checklist.length}
            <p class="progress-copy"><strong>{completedCount} of {checklist.length}</strong> complete <span aria-hidden="true">·</span> {remainingCount} remaining</p>
            <progress max={checklist.length} value={completedCount}><span>{completedCount} of {checklist.length}</span></progress>
            <ul class="checklist">
              {#each checklist as item, index (item.id)}
                <li class:complete={item.done}>
                  {#if !item.done && index === checklist.findIndex((entry) => !entry.done)}
                    <input type="checkbox" id={`check-${item.id}`} checked={item.done} onchange={() => toggleItem(item)} bind:this={firstIncomplete} />
                  {:else}
                    <input type="checkbox" id={`check-${item.id}`} checked={item.done} onchange={() => toggleItem(item)} />
                  {/if}
                  <label for={`check-${item.id}`}>{item.text}</label>
                  <button type="button" class="speak-button" onclick={() => speakItem(item)} aria-label={`Speak checklist item: ${item.text}`} title="Speak item">Speak</button>
                </li>
              {/each}
            </ul>
          {:else}
            <div class="empty-state"><strong>No checklist items yet.</strong><p>Add the steps you need before every stream.</p><button type="button" onclick={openChecklistEditor}>Add checklist items</button></div>
          {/if}
        </section>

        <section class="panel cues-panel" aria-labelledby="cues-heading">
          <div class="panel-heading">
            <div><p class="panel-number">Scenes · C</p><h2 id="cues-heading">Scene cues</h2></div>
            <button type="button" class="quiet-button" onclick={openCuesEditor}>Assign</button>
          </div>
          {#if cues.length}
            <div class="cue-grid">
              {#each cues as cue, index (cue.id)}
                <button type="button" class="cue-key" onclick={() => triggerCue(cue)} disabled={!obs.connected} aria-keyshortcuts={`Control+Shift+${index + 1} Meta+Shift+${index + 1}`}>
                  <span class="key-number" aria-hidden="true">{index + 1}</span>
                  <strong>{cue.label}</strong>
                  <small>Scene: {cue.scene_name}</small>
                </button>
              {/each}
            </div>
            {#if !obs.connected}<p class="inline-note">Scene keys are held until OBS connects. Your assignments are saved.</p>{/if}
          {:else}
            <div class="empty-state"><strong>No scene cues assigned.</strong><p>{obs.connected ? 'Choose from the scenes read from OBS.' : 'Connect OBS, then assign up to nine scene keys.'}</p><button type="button" onclick={openCuesEditor}>Assign scene cues</button></div>
          {/if}
        </section>
      </div>

      <section class="panel metadata-panel" aria-labelledby="metadata-heading">
        <div class="panel-heading">
          <div><p class="panel-number">Metadata · D</p><h2 id="metadata-heading">Platform launch links</h2></div>
          <button type="button" class="quiet-button" onclick={openLinksEditor}>Edit links</button>
        </div>
        <p>Open the platform’s own page to set title and category. This tool does not claim to write metadata or store platform tokens.</p>
        {#if links.length}
          <ul class="link-rail">
            {#each links as link (link.id)}<li><a class="launch-link" href={link.url} target="_blank" rel="noreferrer">{link.label}<span aria-hidden="true">↗</span><span class="sr-only"> (opens in a new tab)</span></a></li>{/each}
          </ul>
        {:else}
          <div class="empty-state inline"><strong>No platform links saved.</strong><button type="button" onclick={openLinksEditor}>Add a link</button></div>
        {/if}
      </section>
    {/if}
  {/if}
</main>

<footer>
  <p>Local-first, free, and built for independent control.</p>
  <nav aria-label="Legal"><a href="/privacy">Privacy</a><a href="/terms">Terms</a><a href="https://github.com/B-Divyesh/sf-stream-access-cues" rel="noreferrer">Source code</a></nav>
  <p class="disclosure">Onboarding illustration generated with the Factory image model; no people or brands depicted.</p>
</footer>

<dialog bind:this={settingsDialog} aria-labelledby="settings-title">
  <form method="dialog" onsubmit={(event) => { event.preventDefault(); saveSettings(); }}>
    <div class="dialog-heading"><div><p class="panel-number">Local connection</p><h2 id="settings-title">OBS WebSocket settings</h2></div><button class="close-button" type="button" onclick={() => settingsDialog.close()} aria-label="Close connection settings">×</button></div>
    <p>In OBS 28 or later, enable the WebSocket server under Tools → WebSocket Server Settings. The default port is 4455.</p>
    <label for="obs-host">OBS host</label>
    <input id="obs-host" bind:value={settingsHost} required autocomplete="off" aria-describedby="host-help" />
    <small id="host-help">Use a host name only, such as 127.0.0.1. In Docker Desktop, try host.docker.internal.</small>
    <label for="obs-port">OBS WebSocket port</label>
    <input id="obs-port" type="number" min="1" max="65535" bind:value={settingsPort} required inputmode="numeric" />
    <label for="obs-password">OBS WebSocket password <span>(optional)</span></label>
    <input id="obs-password" type="password" bind:value={settingsPassword} autocomplete="new-password" aria-describedby="password-help" />
    <small id="password-help">{settings.password_saved ? 'A password is saved. Leave this blank to keep it.' : 'Leave blank if authentication is disabled in OBS.'}</small>
    {#if settings.password_saved}<label class="checkbox-label"><input type="checkbox" bind:checked={clearPassword} /> Remove the saved password</label>{/if}
    {#if settingsError}<p class="form-error" role="alert">{settingsError}</p>{/if}
    <div class="dialog-actions"><button class="quiet-button" type="button" onclick={() => settingsDialog.close()}>Cancel</button><button class="primary-button" type="submit" disabled={saving}>{saving ? 'Saving…' : 'Save and test connection'}</button></div>
  </form>
</dialog>

<dialog bind:this={checklistDialog} aria-labelledby="checklist-dialog-title">
  <form method="dialog" onsubmit={(event) => { event.preventDefault(); saveChecklistEditor(); }}>
    <div class="dialog-heading"><div><p class="panel-number">Preflight controls</p><h2 id="checklist-dialog-title">Edit checklist</h2></div><button class="close-button" type="button" onclick={() => checklistDialog.close()} aria-label="Close checklist editor">×</button></div>
    <p>Items are spoken exactly as written. Put them in the order you perform them.</p>
    <div class="editor-list">
      {#each editChecklist as item, index (item.id)}
        <div class="editor-row"><span class="row-number" aria-hidden="true">{index + 1}</span><div><label for={`edit-check-${item.id}`}>Checklist item {index + 1}</label><input id={`edit-check-${item.id}`} bind:value={item.text} required maxlength="200" /></div><button type="button" class="danger-button" onclick={() => editChecklist = editChecklist.filter((candidate) => candidate.id !== item.id)} aria-label={`Remove checklist item ${index + 1}`}>Remove</button></div>
      {/each}
    </div>
    <button type="button" onclick={() => editChecklist = [...editChecklist, { id: makeId('item'), text: '', done: false }]} disabled={editChecklist.length >= 50}>Add checklist item</button>
    {#if editorError}<p class="form-error" role="alert">{editorError}</p>{/if}
    <div class="dialog-actions"><button class="quiet-button" type="button" onclick={() => checklistDialog.close()}>Cancel</button><button class="primary-button" type="submit" disabled={saving}>{saving ? 'Saving…' : 'Save checklist'}</button></div>
  </form>
</dialog>

<dialog bind:this={cuesDialog} aria-labelledby="cues-dialog-title">
  <form method="dialog" onsubmit={(event) => { event.preventDefault(); saveCuesEditor(); }}>
    <div class="dialog-heading"><div><p class="panel-number">Scene controls</p><h2 id="cues-dialog-title">Assign scene cues</h2></div><button class="close-button" type="button" onclick={() => cuesDialog.close()} aria-label="Close scene cue editor">×</button></div>
    <p>Each cue gets a keyboard shortcut in order, from Control/Command + Shift + 1 through 9.</p>
    <datalist id="obs-scenes">{#each obs.scenes as scene}<option value={scene}></option>{/each}</datalist>
    <div class="editor-list">
      {#each editCues as cue, index (cue.id)}
        <fieldset class="editor-fieldset"><legend>Cue {index + 1}</legend><div><label for={`cue-label-${cue.id}`}>Spoken label</label><input id={`cue-label-${cue.id}`} bind:value={cue.label} required maxlength="60" /></div><div><label for={`cue-scene-${cue.id}`}>Exact OBS scene name</label><input id={`cue-scene-${cue.id}`} bind:value={cue.scene_name} list="obs-scenes" required maxlength="128" /></div><button type="button" class="danger-button" onclick={() => editCues = editCues.filter((candidate) => candidate.id !== cue.id)} aria-label={`Remove cue ${index + 1}`}>Remove cue</button></fieldset>
      {/each}
    </div>
    <button type="button" onclick={() => editCues = [...editCues, { id: makeId('cue'), label: '', scene_name: '' }]} disabled={editCues.length >= 9}>Add scene cue</button>
    {#if !obs.connected}<p class="inline-note">OBS is not connected, so scene suggestions are unavailable. You can still enter the exact name manually.</p>{/if}
    {#if editorError}<p class="form-error" role="alert">{editorError}</p>{/if}
    <div class="dialog-actions"><button class="quiet-button" type="button" onclick={() => cuesDialog.close()}>Cancel</button><button class="primary-button" type="submit" disabled={saving}>{saving ? 'Saving…' : 'Save scene cues'}</button></div>
  </form>
</dialog>

<dialog bind:this={linksDialog} aria-labelledby="links-dialog-title">
  <form method="dialog" onsubmit={(event) => { event.preventDefault(); saveLinksEditor(); }}>
    <div class="dialog-heading"><div><p class="panel-number">Metadata controls</p><h2 id="links-dialog-title">Edit launch links</h2></div><button class="close-button" type="button" onclick={() => linksDialog.close()} aria-label="Close launch link editor">×</button></div>
    <p>Save direct web pages you use to update stream information. Links open in a new tab.</p>
    <div class="editor-list">
      {#each editLinks as link, index (link.id)}
        <fieldset class="editor-fieldset"><legend>Link {index + 1}</legend><div><label for={`link-label-${link.id}`}>Link label</label><input id={`link-label-${link.id}`} bind:value={link.label} required maxlength="80" /></div><div><label for={`link-url-${link.id}`}>Web address</label><input id={`link-url-${link.id}`} type="url" bind:value={link.url} required /></div><button type="button" class="danger-button" onclick={() => editLinks = editLinks.filter((candidate) => candidate.id !== link.id)} aria-label={`Remove link ${index + 1}`}>Remove link</button></fieldset>
      {/each}
    </div>
    <button type="button" onclick={() => editLinks = [...editLinks, { id: makeId('link'), label: '', url: 'https://' }]} disabled={editLinks.length >= 8}>Add launch link</button>
    {#if editorError}<p class="form-error" role="alert">{editorError}</p>{/if}
    <div class="dialog-actions"><button class="quiet-button" type="button" onclick={() => linksDialog.close()}>Cancel</button><button class="primary-button" type="submit" disabled={saving}>{saving ? 'Saving…' : 'Save links'}</button></div>
  </form>
</dialog>

<dialog bind:this={shortcutsDialog} aria-labelledby="shortcuts-title">
  <form method="dialog">
    <div class="dialog-heading"><div><p class="panel-number">Keyboard map</p><h2 id="shortcuts-title">Shortcuts</h2></div><button class="close-button" value="cancel" aria-label="Close shortcut guide">×</button></div>
    <p>Use Control on Windows/Linux or Command on macOS.</p>
    <dl class="shortcut-list"><div><dt><kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>1–9</kbd></dt><dd>Trigger the matching scene cue</dd></div><div><dt><kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>T</kbd></dt><dd>Start or pause the timer</dd></div><div><dt><kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>R</kbd></dt><dd>Open timer reset confirmation</dd></div><div><dt><kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>C</kbd></dt><dd>Focus the next incomplete item</dd></div><div><dt><kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>S</kbd></dt><dd>Speak scene, timer, and next item</dd></div><div><dt><kbd>?</kbd></dt><dd>Open this guide outside a text field</dd></div></dl>
    <div class="dialog-actions"><button class="primary-button" value="cancel">Return to controls</button></div>
  </form>
</dialog>

<dialog bind:this={resetDialog} aria-labelledby="reset-title">
  <form method="dialog">
    <div class="dialog-heading"><div><p class="panel-number">Confirm action</p><h2 id="reset-title">Reset the session timer?</h2></div></div>
    <p>The current elapsed time, {timerText}, will be cleared. This cannot be undone.</p>
    <div class="dialog-actions"><button value="cancel">Keep time</button><button class="danger-solid" type="button" onclick={resetTimer}>Reset to zero</button></div>
  </form>
</dialog>
