<script lang="ts">
  import { onMount } from "svelte";
  import { accountsOverview, dataVersion, startupError, type AccountOverview } from "./lib/api";

  type Route = { kind: "summary" | "budget" | "upcoming" | "all" } | { kind: "account"; id: number };
  let route = $state<Route>({ kind: "summary" });
  let accounts = $state<AccountOverview[]>([]);
  let error = $state<string | null>(null);
  let openAccounts = $derived(accounts.filter((account) => !account.closed));
  let closedAccounts = $derived(accounts.filter((account) => account.closed));
  let total = $derived(openAccounts.reduce((sum, account) => sum + account.balance, 0));
  let heading = $derived(route.kind === "summary" ? "Resumen" :
    route.kind === "budget" ? "Presupuesto" :
    route.kind === "upcoming" ? "Próximos pagos" :
    route.kind === "all" ? "Todas las cuentas" :
    accounts.find((account) => account.id === (route as { id: number }).id)?.name ?? "Cuenta");

  const money = new Intl.NumberFormat("es-MX", { style: "currency", currency: "MXN" });
  const formatMoney = (cents: number) => money.format(cents / 100);

  onMount(() => {
    let stopped = false;
    let timer: ReturnType<typeof setTimeout>;
    let version: number | undefined;

    async function refresh() {
      try {
        const current = await dataVersion();
        if (version !== current) {
          const next = await accountsOverview();
          if (stopped) return;
          accounts = next;
          version = current;
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
      {#if closedAccounts.length}
        <h2>Cerradas</h2>
        {#each closedAccounts as account (account.id)}
          <button class:active={route.kind === "account" && route.id === account.id} onclick={() => route = { kind: "account", id: account.id }}>
            <span class="account-name">{account.name}</span><span class:negative={account.balance < 0} class="balance">{formatMoney(account.balance)}</span>
          </button>
        {/each}
      {/if}
    </nav>
  </aside>
  <main>
    {#if error}
      <div class="card error" role="alert">
        <h1>No se pudo abrir o consultar la base de datos</h1>
        <p>{error}</p>
      </div>
    {:else}
      <div class="card"><h1>{heading}</h1><p>Próximamente</p></div>
    {/if}
  </main>
</div>

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
  aside { width: 290px; flex: none; padding: 24px 16px; border-right: 1px solid var(--border); background: var(--card); overflow-y: auto; }
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
