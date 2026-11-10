<script lang="ts">
  import { onMount } from "svelte";
  import { dialogFocus } from './lib/dialog';
  import Overview from "./Overview.svelte";
  import Entries from "./Entries.svelte";
  import Budget from "./Budget.svelte";
  import Upcoming from "./Upcoming.svelte";
  import Settings from "./Settings.svelte";
  import { accountsOverview, dataVersion, startupError, today, createAccount, type AccountOverview } from "./lib/api";

  type Route = { kind: "summary" | "budget" | "upcoming" | "settings" | "all" } | { kind: "account"; id: number };
  let route = $state<Route>({ kind: "summary" });
  let accounts = $state<AccountOverview[]>([]);
  let error = $state<string | null>(null);
  let version = $state("");
  let forceRefresh = $state(0);
  let month = $state(0), currentMonth = $state(0), addingAccount = $state(false), accountName = $state(''), accountError = $state('');
  async function addAccount() { try { await createAccount(accountName); addingAccount = false; accountName = ''; forceRefresh++; } catch (e) { accountError = String(e); } }
  let openAccounts = $derived(accounts.filter((account) => !account.closed));
  let closedAccounts = $derived(accounts.filter((account) => account.closed));
  let total = $derived(openAccounts.reduce((sum, account) => sum + account.balance, 0));
  let heading = $derived(route.kind === "summary" ? "Resumen" :
    route.kind === "budget" ? "Presupuesto" :
    route.kind === "upcoming" ? "Próximos pagos" :
    route.kind === "settings" ? "Ajustes" :
    route.kind === "all" ? "Todas las cuentas" :
    accounts.find((account) => account.id === (route as { id: number }).id)?.name ?? "Cuenta");

  const money = new Intl.NumberFormat("es-MX", { style: "currency", currency: "MXN" });
  const formatMoney = (cents: number) => money.format(cents / 100);

  onMount(() => {
    let stopped = false;
    let timer: ReturnType<typeof setTimeout>;
    let observedVersion: string | undefined;
    let observedForce = -1;

    async function refresh() {
      try {
        const current = await dataVersion();
        if (observedVersion !== current || observedForce !== forceRefresh) {
          const next = await accountsOverview();
          if (stopped) return;
          accounts = next;
          observedVersion = current;
          observedForce = forceRefresh;
          version = `${current}:${forceRefresh}`;
        }
        error = null;
      } catch (cause) {
        if (!stopped) error = String(cause);
      }
      if (!stopped) timer = setTimeout(refresh, 2000);
    }

    async function start() {
      try {
        const failure = await startupError();
        if (stopped) return;
        if (failure) error = failure;
        else await refresh();
      } catch (cause) {
        if (!stopped) error = String(cause);
      }
    }
    void today().then(d => { currentMonth = Number(d.slice(0, 7).replace('-', '')); month = currentMonth; }).catch(e => error = String(e));
    void start();
    return () => { stopped = true; clearTimeout(timer); };
  });
</script>

<div class="shell">
  <aside>
    <header>
      <div class="logo" aria-hidden="true">F</div>
      <div><strong>Finanzas</strong><small>Tu dinero, en orden</small></div>
    </header>
    <nav aria-label="Navegación principal">
      <button class:active={route.kind === "summary"} onclick={() => route = { kind: "summary" }}>Resumen</button>
      <button class:active={route.kind === "budget"} onclick={() => route = { kind: "budget" }}>Presupuesto</button>
      <button class:active={route.kind === "upcoming"} onclick={() => route = { kind: "upcoming" }}>Próximos pagos</button>
      <h2>Cuentas</h2>
      <button class:active={route.kind === "all"} onclick={() => route = { kind: "all" }}>
        <span>Todas</span><span class:negative={total < 0} class="balance">{formatMoney(total)}</span>
      </button>
      {#each openAccounts as account (account.id)}
        <button class:active={route.kind === "account" && route.id === account.id} onclick={() => route = { kind: "account", id: account.id }}>
          <span class="account-name">{account.name}</span><span class:negative={account.balance < 0} class="balance">{formatMoney(account.balance)}</span>
        </button>
      {/each}
      <button onclick={() => {accountName = ''; accountError = ''; addingAccount = true;}}>＋ Nueva cuenta</button>
      {#if closedAccounts.length}
        <h2>Cerradas</h2>
        {#each closedAccounts as account (account.id)}
          <button class:active={route.kind === "account" && route.id === account.id} onclick={() => route = { kind: "account", id: account.id }}>
            <span class="account-name">{account.name}</span><span class:negative={account.balance < 0} class="balance">{formatMoney(account.balance)}</span>
          </button>
        {/each}
      {/if}
    </nav>
    <button class="settings" class:active={route.kind === 'settings'} aria-label="Ajustes" onclick={() => route = {kind:'settings'}}>⚙ Ajustes</button>
  </aside>
  <main>
    {#if error}
      <div class="card error" role="alert">
        <h1>No se pudo abrir o consultar la base de datos</h1>
        <p>{error}</p>
      </div>
    {:else if route.kind === "summary"}
      <Overview {version} bind:month {currentMonth} refresh={() => {forceRefresh++;}}/>
    {:else if route.kind === 'budget'}
      <Budget {version} bind:month {currentMonth} refresh={() => {forceRefresh++;}}/>
    {:else if route.kind === 'upcoming'}
      <Upcoming {version} accounts={openAccounts} refresh={() => {forceRefresh++;}}/>
    {:else if route.kind === 'settings'}
      <Settings {version} refresh={() => {forceRefresh++;}}/>
    {:else if route.kind === "all" || route.kind === "account"}
      <Entries accounts={openAccounts} accountId={route.kind === "account" ? route.id : undefined} {heading} {version} refresh={() => { forceRefresh++; }}/>
    {:else}
      <div class="card"><h1>{heading}</h1><p>Próximamente</p></div>
    {/if}
  </main>
</div>
{#if addingAccount}<div class="backdrop"><div class="modal" use:dialogFocus role="dialog" aria-modal="true" aria-labelledby="account-title" tabindex="-1" onkeydown={e => {if(e.key === 'Escape') addingAccount=false;}}><h2 id="account-title">Nueva cuenta</h2><form onsubmit={e => {e.preventDefault(); void addAccount();}}><label>Nombre <input bind:value={accountName} required/></label>{#if accountError}<p role="alert">{accountError}</p>{/if}<div class="modal-actions"><button type="button" onclick={() => addingAccount=false}>Cancelar</button><button type="submit">Crear</button></div></form></div></div>{/if}

<style>
  :global(:root) {
    font-family: system-ui, sans-serif;
    color: #1c2933;
    background: #f4f7fa;
    --petroleum: #155e75; --emerald: #147d59; --coral: #be493d; --amber: #986000;
    --canvas: #f4f7fa; --card: #fff; --border: #dee7ed; --hero: #155e75;
    --expense-fill: #be493d; --income-fill: #147d59; --muted: #5b6d7b;
    color-scheme: light dark;
  }
  :global(*) { box-sizing: border-box; }
  :global(body) { margin: 0; }
  .shell { display: flex; min-height: 100vh; background: var(--canvas); }
  aside { width: 290px; flex: none; padding: 24px 16px; border-right: 1px solid var(--border); background: var(--card); overflow-y: auto; display:flex; flex-direction:column; }
  .settings { margin-top:auto; }
  .backdrop { position:fixed; inset:0; background:#0009; display:grid; place-items:center; z-index:10; }
  .modal { background:var(--card); padding:24px; border-radius:16px; min-width:300px; }
  .modal form { display:grid; gap:16px; } .modal input { font:inherit; max-width:100%; }
  .modal-actions { display:flex; gap:8px; } .modal-actions button { border:1px solid var(--border); }
  header { display: flex; align-items: center; gap: 12px; margin: 0 8px 28px; }
  header strong { display: block; font-size: 1.2rem; }
  small { display: block; color: var(--muted); margin-top: 2px; }
  .logo { display: grid; place-items: center; width: 42px; height: 42px; border-radius: 12px; background: var(--hero); color: white; font-weight: 700; }
  nav { display: flex; flex-direction: column; gap: 3px; }
  nav h2 { margin: 22px 10px 7px; font-size: .75rem; text-transform: uppercase; letter-spacing: .06em; color: var(--muted); }
  button { display: flex; justify-content: space-between; gap: 8px; width: 100%; padding: 9px 10px; border: 0; border-radius: 8px; background: transparent; color: inherit; text-align: left; font: inherit; cursor: pointer; }
  button:hover, button.active { background: var(--canvas); color: var(--petroleum); }
  button.active { font-weight: 600; }
  button:focus-visible { outline: 2px solid var(--petroleum); outline-offset: 1px; }
  .account-name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .balance { white-space: nowrap; font-variant-numeric: tabular-nums; }
  .negative { color: var(--coral); }
  main { flex: 1; min-width: 0; padding: 32px; }
  .card { padding: 26px; border: 1px solid var(--border); border-radius: 18px; background: var(--card); }
  h1 { margin: 0; font-size: 1.75rem; }
  p { color: var(--muted); overflow-wrap: anywhere; }
  .error h1 { color: var(--coral); font-size: 1.25rem; }
  @media (prefers-color-scheme: dark) {
    :global(:root) {
      color: #edf3f6; --petroleum: #67c6df; --emerald: #6ddbad; --coral: #ff9a8f; --amber: #f5c46b;
      --canvas: #151d27; --card: #1e2936; --border: #354453; --muted: #afbec8;
    }
  }
</style>
