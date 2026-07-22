<script lang="ts">
  import { tick } from "svelte";
  import { ChevronDown, ChevronUp, Plus, X, MessageSquarePlus, ChevronRight, FolderOpen, HardDrive } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";
  import { workspace } from "../../state/workspace.svelte";
  import { stt } from "../../state/stt.svelte";
  import MessageBubble from "./MessageBubble.svelte";
  import StreamTurn from "./stream/StreamTurn.svelte";
  import PlanHud from "./stream/PlanHud.svelte";
  import ActivityHud from "./stream/ActivityHud.svelte";
  import { uiPrefs } from "$lib/state/ui-prefs.svelte";
  import AssistantWelcome from "./AssistantWelcome.svelte";
  import Composer from "./Composer.svelte";
  import { leafName } from "$lib/utils/path";

  import { tooltip } from "$lib/actions/tooltip";
  let {
    tabId,
    focused,
    paneIdx,
  }: { tabId: string | null; focused: boolean; paneIdx: number } = $props();

  // Park the jump-to-latest button just above the composer no matter how tall it
  // grows (queue rail / attachments / enhance bar). A fixed px offset hid the
  // button behind a tall composer.
  let composerSlotEl = $state<HTMLDivElement | undefined>();
  let composerH = $state(72);
  $effect(() => {
    const el = composerSlotEl;
    if (!el) return;
    // Composer growth also shifts the scroller's floor — keep a latched
    // transcript pinned through it (instant: layout shift, not stream motion).
    const ro = new ResizeObserver(() => { composerH = el.offsetHeight; if (stickToBottom) pinToBottom(true); });
    ro.observe(el);
    return () => ro.disconnect();
  });

  const tab = $derived(assistant.tabFor(tabId));
  const messages = $derived(tab?.messages ?? []);
  const streaming = $derived(tab?.streaming ?? false);
  const lastError = $derived(tab?.lastError ?? null);
  const showEmpty = $derived(messages.length === 0);
  const needsAuth = $derived(assistant.auth?.pill === "red");
  // Notices are session-global; only show on the focused pane to avoid dup banners.
  const showNotice = $derived(focused && !!assistant.lastNotice);
  // Per-tab error renders in whichever pane owns the erroring tab, focused or
  // not — otherwise a background-pane send-failure is silent until refocus.
  const showError = $derived(!!lastError);
  // Auth-class errors become an *actionable* recovery banner instead of a dead
  // wall of text. A rejected API key routes to Settings (where it's cleared);
  // a rejected/expired `claude login` routes to in-app sign-in.
  const isKeyError = $derived(
    !!lastError && /api key was rejected|configured api key/i.test(lastError),
  );
  const isAuthError = $derived(
    !!lastError &&
      (isKeyError ||
        /\b401\b|authentication failed|not logged in|claude login|session was rejected|sign in there/i.test(
          lastError,
        )),
  );
  // Per-pane status chip — own tab's ctx%, model, cost — independent of focus.
  const paneCtxPct = $derived(tab ? assistant.ctxPctFor(tab) : 0);
  const paneCtxTone = $derived(
    paneCtxPct >= 90 ? "red" : paneCtxPct >= 70 ? "yellow" : "ok",
  );
  const paneModel = $derived(tab?.lastModelId ?? null);
  const paneCost = $derived(tab?.totalCostUsd ?? null);
  // This pane's project folder (its own per-tab root, else the global default) —
  // each pane can run in a DIFFERENT folder, so the head surfaces the basename +
  // a click-to-change picker. "—" when nothing is set (pure conversational).
  const paneRoot = $derived(tab ? assistant.effectiveRoot(tab) : null);
  const paneFolder = $derived(paneRoot ? leafName(paneRoot) : null);
  const paneChipTitle = $derived.by(() => {
    if (!tab) return "";
    const w = assistant.ctxWindowFor(tab);
    const used = Math.round((paneCtxPct / 100) * w);
    const lines = [
      `Ctx: ${used.toLocaleString()} / ${w.toLocaleString()} (${paneCtxPct.toFixed(1)}%)`,
    ];
    // Strip the `[1m]` window-selector suffix the backend appends to the CLI
    // `--model` arg for 1M-gated Sonnets (turn.rs/config.rs::cli_model_arg) — it
    // echoes back in lastModelId but is an internal routing artifact, not a model
    // name the user should see.
    if (paneModel) lines.push(`Model: ${paneModel.replace(/\[1m\]/i, "")}`);
    if (paneCost != null) lines.push(`Cost: ${paneCost.toFixed(4)}`);
    return lines.join("\n");
  });
  // Human-friendly label for the pane header so a split is identifiable by its
  // conversation, not just a number. Mirrors healthAlerts.tabTitle.
  const paneTitle = $derived.by(() => {
    if (!tabId || !tab) return "Empty pane";
    if (tab.convoTitle) return tab.convoTitle;
    const first = tab.messages.find((m) => m.role === "user");
    const text = first?.blocks
      .map((b) => (b.type === "text" ? b.text : ""))
      .join("")
      .trim()
      .replace(/\s+/g, " ");
    if (!text) return "New chat";
    return text.length > 48 ? text.slice(0, 48) + "…" : text;
  });

  let scrollEl = $state<HTMLDivElement | undefined>();
  let messagesEl = $state<HTMLDivElement | undefined>();
  let stickToBottom = $state(true);
  let scrolledTop = $state(false);
  let lastTabId: string | null = null;

  // ── turn-rail: right-edge exchange scrubber (shows at 2+ exchanges) ────────
  // One tick per user message (the anchor of an exchange). The active tick is
  // the last user bubble whose top has scrolled past the viewport top.
  const exchangeCount = $derived(messages.filter((m) => m.role === "user").length);
  let activeXch = $state(0);

  function userBubbles(): HTMLElement[] {
    if (!scrollEl) return [];
    return [...scrollEl.querySelectorAll<HTMLElement>('.bubble[data-role="user"]')];
  }
  function updateActiveXch() {
    if (!scrollEl) return;
    const top = scrollEl.getBoundingClientRect().top;
    const bs = userBubbles();
    let idx = 0;
    for (let k = 0; k < bs.length; k++) {
      if (bs[k].getBoundingClientRect().top - top <= 48) idx = k;
      else break;
    }
    activeXch = idx;
  }
  function scrollToXch(i: number) {
    const bs = userBubbles();
    const el = bs[i];
    if (!el || !scrollEl) return;
    const offset =
      el.getBoundingClientRect().top - scrollEl.getBoundingClientRect().top + scrollEl.scrollTop - 16;
    scrollEl.scrollTo({ top: Math.max(0, offset), behavior: "smooth" });
  }
  function stepXch(dir: -1 | 1) {
    scrollToXch(Math.min(exchangeCount - 1, Math.max(0, activeXch + dir)));
  }

  // Programmatic scrolls must never un-latch stickToBottom. Scroll events echo
  // ASYNCHRONOUSLY after a pin — if content grew in between, the handler reads
  // gap > 80 from our own echo and kills follow (the classic dead-autoscroll
  // race during bursty streams / conversation hydration). So every pin records
  // the position it set (pinEcho) and its echo is ignored; smooth glides get a
  // latch-only window (glideUntil) since their intermediate positions are
  // unpredictable. Only scroll positions WE didn't create can un-latch.
  let glideUntil = 0;
  let pinEcho = -1;

  // Streaming follow glides instead of teleporting: a rAF lerp re-reads the
  // target each frame, so bursty content growth reads as one continuous motion.
  // pinEcho tracks every frame we write, so our own scroll echoes stay latched
  // while a real user scroll (position ≠ pinEcho) still un-latches instantly.
  // Big jumps snap — chasing three screens reads worse than a cut.
  let smoothRaf = 0;
  const paneReducedMotion =
    typeof window !== "undefined" &&
    window.matchMedia?.("(prefers-reduced-motion: reduce)").matches === true;

  function pinToBottom(instant = false) {
    if (!scrollEl) return;
    if (instant || paneReducedMotion) {
      if (smoothRaf) { cancelAnimationFrame(smoothRaf); smoothRaf = 0; }
      scrollEl.scrollTop = scrollEl.scrollHeight;
      pinEcho = scrollEl.scrollTop; // post-clamp actual position
      return;
    }
    if (smoothRaf) return; // glide already running — it re-reads the target
    const step = () => {
      smoothRaf = 0;
      if (!scrollEl || !stickToBottom) return;
      const delta = scrollEl.scrollHeight - scrollEl.clientHeight - scrollEl.scrollTop;
      if (Math.abs(delta) < 0.5) { pinEcho = scrollEl.scrollTop; return; }
      scrollEl.scrollTop = delta > 900 ? scrollEl.scrollHeight : scrollEl.scrollTop + delta * 0.2;
      pinEcho = scrollEl.scrollTop;
      smoothRaf = requestAnimationFrame(step);
    };
    smoothRaf = requestAnimationFrame(step);
  }
  $effect(() => () => { if (smoothRaf) cancelAnimationFrame(smoothRaf); });

  function onScroll() {
    if (!scrollEl) return;
    const gap = scrollEl.scrollHeight - scrollEl.scrollTop - scrollEl.clientHeight;
    if (Math.abs(scrollEl.scrollTop - pinEcho) < 2) {
      // echo of our own pin — content may have grown since; stay latched
    } else if (performance.now() < glideUntil) {
      if (gap < 80) glideUntil = 0; // glide arrived — resume normal tracking
    } else {
      stickToBottom = gap < 80;
    }
    scrolledTop = scrollEl.scrollTop > 8;
    updateActiveXch();
    if (tabId) assistant.setTabScroll(tabId, scrollEl.scrollTop);
  }

  $effect(() => {
    if (!messagesEl) return;
    const ro = new ResizeObserver(() => {
      // Background-pane streams: only force-stick when the user was already
      // at the bottom — don't yank the scroll on a pane they're reading
      // mid-history just because a sibling pane is streaming.
      if (stickToBottom) pinToBottom();
    });
    ro.observe(messagesEl);
    return () => ro.disconnect();
  });

  $effect(() => {
    const _len = messages.length;
    const _streaming = streaming;
    void _len; void _streaming;
    void tick().then(() => {
      if (stickToBottom) pinToBottom();
    });
  });

  $effect(() => {
    const cur = tabId;
    if (cur === lastTabId) return;
    lastTabId = cur;
    if (!cur || !scrollEl) return;
    const cached = assistant.getTabScroll(cur);
    void tick().then(() => {
      if (!scrollEl) return;
      if (cached != null) {
        // Instant — a smooth glide here animates from the PREVIOUS tab's
        // scroll position across the new tab's content. Pure noise.
        scrollEl.scrollTop = cached;
        pinEcho = scrollEl.scrollTop;
        const gap = scrollEl.scrollHeight - cached - scrollEl.clientHeight;
        stickToBottom = gap < 80;
      } else {
        // Instant — gliding from the top of a freshly opened tab is noise.
        pinToBottom(true);
        stickToBottom = true;
      }
    });
  });

  function jumpToLatest() {
    if (!scrollEl) return;
    stickToBottom = true;
    glideUntil = performance.now() + 700;
    scrollEl.scrollTo({ top: scrollEl.scrollHeight, behavior: "smooth" });
  }

  // ── Composer FLIP: home (centered hero) → conversation (docked bottom) ────
  // The composer-host is a persistent node in `.csurf-col` (outside the
  // home/convo {#if}), so the SAME element survives the first send. We snapshot
  // its rect before the send mutates state, then invert+animate to the docked
  // position once the layout settles (mirrors the comp `chat.jsx` FLIP).
  let flipFirst: DOMRect | null = null;
  let prevEmpty: boolean | null = null;
  let flipPrevTabId: string | null = null;
  let welcomeEl = $state<HTMLDivElement | null>(null);
  let welcomeExitRect: DOMRect | null = null;

  function handleSend(text: string) {
    if (showEmpty && composerSlotEl) {
      flipFirst = composerSlotEl.getBoundingClientRect();
      welcomeExitRect = welcomeEl?.getBoundingClientRect() ?? null;
    }
    // Sending always re-latches follow — your own message must be on screen.
    stickToBottom = true;
    assistant.send(text, tabId);
  }

  // ── Engaged posture: the home surface reacts the moment you engage the
  // composer (focus it, or start dictation) — the welcome drifts away and the
  // composer FLIPs DOWN to its working position before anything is sent.
  // Leaving it (blur with an empty draft, no dictation) floats everything
  // back. Send from the engaged posture is then just a small settle.
  let engaged = $state(false);
  // `starting` included: mic init can take a beat, and the focusout guard
  // below would otherwise disengage (composer bounces back up) in the gap
  // between the mic click and `recording` flipping true.
  const sttHere = $derived((stt.recording || stt.starting) && stt.targetTabId === tabId);

  function setEngaged(on: boolean) {
    if (engaged === on) return;
    if (!showEmpty) { engaged = on; return; } // no motion outside the hero
    if (composerSlotEl) flipFirst = composerSlotEl.getBoundingClientRect();
    if (on) welcomeExitRect = welcomeEl?.getBoundingClientRect() ?? null;
    engaged = on;
    runComposerFlip();
  }

  // Dictation engages even if the click never focused the textarea.
  $effect(() => { if (sttHere) setEngaged(true); });

  function onHostFocusIn() { setEngaged(true); }
  function onHostFocusOut() {
    // rAF: menu clicks bounce focus through <body> for a frame (both bar
    // popovers refocus the textarea on open); only disengage once focus has
    // truly left the composer and there's nothing in flight to come back to.
    requestAnimationFrame(() => {
      const ae = document.activeElement;
      if (composerSlotEl?.contains(ae)) return;
      if ((tab?.draft ?? "").trim().length > 0 || sttHere) return;
      setEngaged(false);
    });
  }

  // Welcome exit — engage + send paths (rect captured right before the state
  // flip; every other showEmpty flip, e.g. resuming a chat, leaves it null
  // and exits instantly like before). Pins the block at its exact pre-flip
  // viewport spot with position:fixed so it leaves the layout flow at once —
  // the composer FLIP below then measures its true docked target — and
  // fades/drifts up while the composer glides down past it.
  function welcomeOut(node: HTMLElement) {
    const r = welcomeExitRect;
    welcomeExitRect = null;
    if (!r || window.matchMedia("(prefers-reduced-motion: reduce)").matches) return { duration: 0 };
    node.style.position = "fixed";
    node.style.top = `${r.top}px`;
    node.style.left = `${r.left}px`;
    node.style.width = `${r.width}px`;
    node.style.margin = "0";
    node.style.pointerEvents = "none";
    return {
      duration: 200,
      css: (t: number) => `opacity: ${t}; transform: translateY(${(1 - t) * -10}px)`,
    };
  }

  // Welcome re-entry on disengage — fades back down into place as the
  // composer floats up to meet it.
  function welcomeIn(node: HTMLElement) {
    void node;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return { duration: 0 };
    return {
      duration: 220,
      css: (t: number) => `opacity: ${t}; transform: translateY(${(1 - t) * -10}px)`,
    };
  }

  function runComposerFlip() {
    const host = composerSlotEl;
    const first = flipFirst;
    flipFirst = null;
    if (!host || !first) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    void tick().then(() => {
      if (!host) return;
      const last = host.getBoundingClientRect();
      const dy = Math.round(first.top - last.top);
      if (!dy) return;
      host.style.transition = "none";
      host.style.transform = `translateY(${dy}px)`;
      void host.offsetHeight;
      host.style.transition = "transform 460ms cubic-bezier(0.22, 1, 0.36, 1)";
      host.style.transform = "translateY(0)";
      let flipTimer: ReturnType<typeof setTimeout> | undefined;
      const clear = () => {
        if (flipTimer) clearTimeout(flipTimer);
        host.removeEventListener("transitionend", clear);
        host.style.transition = ""; host.style.transform = "";
      };
      host.addEventListener("transitionend", clear, { once: true });
      flipTimer = setTimeout(clear, 540);
    });
  }

  $effect(() => {
    const empty = showEmpty;
    // Re-baseline on tab switch so the previous tab's empty-state can't read as
    // a home→convo transition and fire a spurious FLIP on the newly-shown tab.
    if (tabId !== flipPrevTabId) {
      flipPrevTabId = tabId;
      prevEmpty = empty;
      flipFirst = null;
      // Re-derive (not just reset) posture: a tab switch keeps the composer's
      // focus, so no focusin will re-fire — focus inside = still engaged.
      engaged = !!(composerSlotEl && composerSlotEl.contains(document.activeElement));
      return;
    }
    const was = prevEmpty;
    prevEmpty = empty;
    if (was === null || was === empty) return;
    if (was && !empty) runComposerFlip();
    else flipFirst = null;
  });

  function onPaneClick() {
    if (!focused) assistant.setFocusedPane(paneIdx);
  }

  // ── drag-from-tabsbar drop targets ───────────────────────────────────────
  // Pane is the sole drop target — overlays are purely visual. This avoids
  // the Chromium "not allowed" cursor that latches when dragover briefly
  // passes over an ancestor without a preventDefault chain (e.g. .layout
  // between .tabsbar and .pane). One container, continuous preventDefault.
  //
  // Half detection: in single-pane mode we compute which half of the pane
  // the cursor is over from `e.clientX` so drop assigns to pane 0 (left) or
  // pane 1 (right). In split mode the whole pane is one zone tied to
  // `paneIdx`.
  const dragging = $derived(assistant.draggingTabId ?? assistant.draggingProjectRoot);
  // Project drags get project-flavored drop labels ("Open here / Open → left").
  const dragIsProject = $derived(assistant.draggingProjectRoot != null);
  let hoverHalf = $state<"left" | "right" | "full" | null>(null);
  let paneEl = $state<HTMLDivElement | undefined>();

  function computeHalf(e: DragEvent): "left" | "right" | "full" {
    if (assistant.splitActive) return "full";
    const target = paneEl;
    if (!target) return "left";
    const rect = target.getBoundingClientRect();
    return e.clientX < rect.left + rect.width / 2 ? "left" : "right";
  }

  function onClosePane(e: MouseEvent) {
    e.stopPropagation();
    assistant.closePane(paneIdx);
  }

  // Per-pane folder picker — sets THIS pane's tab root only (pickTabFolder never
  // touches the global default), so two panes can run in different projects at
  // once. Focus first so the @-mention/branch caches refresh against this pane.
  function onPickPaneFolder(e: MouseEvent) {
    e.stopPropagation();
    if (!focused) assistant.setFocusedPane(paneIdx);
    void assistant.pickTabFolder(tabId);
  }

  // Empty-pane actions — surface New chat / Close pane / Recent picks so
  // an unassigned slot is actionable instead of just saying "No tab in this
  // pane". Each handler focuses this pane first so newTab / openTab assigns
  // here (both rely on focusedPaneIdx via assignFocusedPane).
  function onEmptyNew() {
    if (!focused) assistant.setFocusedPane(paneIdx);
    void assistant.newTab();
  }
  function onEmptyClosePane() {
    assistant.closePane(paneIdx);
  }
  function onEmptyOpenRecent(id: string) {
    if (!focused) assistant.setFocusedPane(paneIdx);
    void assistant.openTab(id);
  }
  // Skip convos already visible in another pane — moving a tab cross-pane
  // from this picker would yank it from a pane the user can see.
  const emptyRecents = $derived.by(() => {
    if (tabId) return [];
    const inPanes = new Set(
      assistant.panes.map((p) => p.tabId).filter((x): x is string => !!x),
    );
    return assistant.conversations
      .filter((c) => !inPanes.has(c.id))
      .slice(0, 3);
  });

  function onPaneDragOver(e: DragEvent) {
    // Always preventDefault when a tab OR a project chip is being dragged —
    // without this on BOTH dragenter and dragover, Chromium shows "no-drop".
    if (!assistant.draggingTabId && !assistant.draggingProjectRoot) return;
    e.preventDefault();
    // dropEffect MUST be compatible with the source's effectAllowed or the
    // drop is rejected (no-drop cursor, no drop event fires). Project chips
    // start the drag with effectAllowed="copy"; tab drags use "move".
    if (e.dataTransfer) e.dataTransfer.dropEffect = assistant.draggingProjectRoot ? "copy" : "move";
    const next = computeHalf(e);
    if (hoverHalf !== next) hoverHalf = next;
  }

  function onPaneDragLeave(e: DragEvent) {
    // Only clear if the cursor actually left the pane (not just crossed
    // into a child element). relatedTarget is null when leaving the
    // browser window or moving outside any tracked element.
    const to = e.relatedTarget as Node | null;
    if (to && paneEl && paneEl.contains(to)) return;
    hoverHalf = null;
  }

  function onPaneDrop(e: DragEvent) {
    const tabDrag = assistant.draggingTabId;
    const projRoot = assistant.draggingProjectRoot;
    if (!tabDrag && !projRoot) return;
    e.preventDefault();
    const half = computeHalf(e);
    hoverHalf = null;
    const targetPane: number = assistant.splitActive
      ? paneIdx
      : half === "left" ? 0 : 1;
    if (projRoot) {
      // Project chip → open it as a fresh chat in the dropped-on pane. In
      // single-pane mode, dropping on the empty half grows the split; the
      // store's openProjectInPane handles the cap-aware grow.
      const splitNew = !assistant.splitActive && targetPane >= assistant.panes.length;
      void assistant.openProjectInPane(projRoot, { paneIdx: targetPane, splitNew });
      return;
    }
    if (tabDrag) assistant.dropTabIntoPane(tabDrag, targetPane);
  }
</script>

<div class="pane-shell" class:split={assistant.splitActive}>
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
  bind:this={paneEl}
  class="pane"
  class:focused
  class:split={assistant.splitActive}
  role="region"
  aria-label={focused ? "Focused chat pane" : "Inactive chat pane"}
  onclick={onPaneClick}
  onkeydown={(e) => { if (e.key === "Enter") onPaneClick(); }}
  ondragenter={onPaneDragOver}
  ondragover={onPaneDragOver}
  ondragleave={onPaneDragLeave}
  ondrop={onPaneDrop}
  tabindex={focused ? -1 : 0}
>
  {#if assistant.splitActive}
    <div class="pane-head" class:focused>
      <span class="pane-label" use:tooltip={`Pane ${paneIdx + 1} of ${assistant.panes.length}`}>{paneIdx + 1}</span>
      <span class="pane-head-title" use:tooltip={paneTitle}>{paneTitle}</span>
      {#if tabId && tab}
        {@const localMode = !paneFolder && assistant.isLocalMode}
        <button
          class="pane-folder"
          class:local-mode={localMode}
          type="button"
          use:tooltip={paneRoot
            ? `Project: ${paneRoot}\nClick to change this pane's folder`
            : localMode
              ? `Working locally — turns run in ${assistant.localScratchPath}\nClick to open a project folder`
              : "Set this pane's project folder"}
          aria-label={localMode ? "Working locally — click to open a project folder" : "Change pane folder"}
          onclick={onPickPaneFolder}
        >
          {#if localMode}<HardDrive size={11}/>{:else}<FolderOpen size={11}/>{/if}
          <span class="pane-folder-name">{paneFolder ?? (localMode ? "Local" : "No folder")}</span>
        </button>
      {/if}
      {#if streaming}
        <span class="pane-live" use:tooltip={"This pane is working"}><span class="pane-live-dot"></span>working</span>
      {/if}
      {#if tabId && tab}
        <span class="pane-ctx-chip" data-tone={paneCtxTone} use:tooltip={paneChipTitle}>
          <span class="pane-ctx-bar"><span class="pane-ctx-fill" style="width: {Math.min(100, paneCtxPct)}%"></span></span>
          <span class="pane-ctx-pct">{Math.round(paneCtxPct)}%</span>
          {#if paneCost != null}
            <span class="pane-cost">${paneCost.toFixed(2)}</span>
          {/if}
        </span>
      {/if}
      <button
        class="pane-close"
        type="button"
        use:tooltip={"Close this pane"}
        aria-label="Close pane"
        onclick={onClosePane}
      >
        <X size={11}/>
      </button>
    </div>
  {/if}

  {#if !tabId && assistant.splitActive}
    <div class="scroll">
      <div class="pane-empty">
        <div class="pane-empty-card">
          <div class="pane-empty-mark"><MessageSquarePlus size={20}/></div>
          <div class="pane-empty-title">Empty pane</div>
          <div class="pane-empty-hint">
            <!-- No tabs BAR exists since the redesign — chats drag in from the sidebar. -->
            Start a fresh chat here, or drag a chat in from the sidebar.
          </div>
          <div class="pane-empty-actions">
            <button class="btn primary sm" type="button" onclick={onEmptyNew}>
              <Plus size={12}/> New chat
            </button>
            {#if assistant.panes.length > 1}
              <button class="btn ghost sm" type="button" onclick={onEmptyClosePane}>
                <X size={12}/> Close pane
              </button>
            {/if}
          </div>
          {#if emptyRecents.length > 0}
            <div class="pane-empty-recent">
              <div class="pane-empty-recent-label">RESUME</div>
              {#each emptyRecents as c (c.id)}
                <button
                  class="pane-empty-recent-row"
                  type="button"
                  onclick={() => onEmptyOpenRecent(c.id)}
                  use:tooltip={c.title}
                >
                  <span class="pane-empty-recent-title">{c.title}</span>
                  <span class="pane-empty-recent-meta">{c.messageCount} msg</span>
                  <ChevronRight class="pane-empty-recent-chev" size={13}/>
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    </div>
  {:else}
    <!-- Merged surface: HOME (centered hero) ⇄ CONVERSATION. The composer-host
         is a persistent node so it FLIPs from center → bottom on the first send
         (mirrors comp `chat.jsx`). Welcome/stream swap inside; alerts +
         composer stay below the content within the same column. -->
    <div class="csurf-col" class:is-home={showEmpty} class:is-convo={!showEmpty} class:engaged>
      {#if showEmpty && !engaged}
        <div class="welcome-wrap" bind:this={welcomeEl} in:welcomeIn out:welcomeOut>
          <AssistantWelcome {needsAuth} {tabId} />
        </div>
      {:else if !showEmpty}
        <div class="stream" bind:this={scrollEl} onscroll={onScroll} onwheel={() => (glideUntil = 0)}>
          <div class="messages" bind:this={messagesEl}>
            {#each messages as m, mi (m.id)}
              {#if uiPrefs.streamMode && m.role === "assistant"}
                <StreamTurn
                  message={m}
                  {tab}
                  isLast={mi === messages.length - 1}
                  streaming={streaming && mi === messages.length - 1}
                />
              {:else}
                <MessageBubble
                  message={m}
                  {tab}
                  {tabId}
                  isLast={mi === messages.length - 1}
                  streaming={streaming
                    && mi === messages.length - 1
                    && m.role === "assistant"}
                />
              {/if}
            {/each}
          </div>
        </div>
        <!-- Pinned HUD stack — plan + agent periscopes float over the stream's
             top edge so live state stays glanceable while the timeline scrolls
             past. The stack owns centering/width; each HUD manages its own
             visibility. NOT {#key}-ed on the tab: each HUD resets its own
             per-tab state when the `tab` prop swaps (see PlanHud) — a keyed
             remount here tore down a focused bar on Ctrl+Tab and dropped
             keyboard focus to <body>. -->
        <div class="hud-stack">
          <PlanHud {tab} {streaming} />
          <ActivityHud {tab} {tabId} {streaming} />
        </div>
      {/if}

      {#if showError || showNotice}
        <div class="alerts">
          {#if showNotice}
            <button class="alert notice" type="button" onclick={() => assistant.dismissNotice()} use:tooltip={"Click to dismiss"}>
              <span class="notice-icon">ℹ</span>
              <span class="notice-text">{assistant.lastNotice}</span>
            </button>
          {/if}
          {#if showError && isAuthError}
            <div class="alert error recovery">
              <span class="notice-icon">⚠</span>
              <div class="recovery-body">
                <span class="notice-text">{lastError}</span>
                <div class="recovery-actions">
                  {#if isKeyError}
                    <button class="recovery-btn primary" type="button" onclick={() => workspace.setActive("settings")}>
                      Open Settings
                    </button>
                  {:else}
                    <button
                      class="recovery-btn primary"
                      type="button"
                      disabled={assistant.loginInProgress}
                      onclick={() => assistant.startLogin()}
                    >
                      {assistant.loginInProgress ? "Signing in…" : "Sign in"}
                    </button>
                  {/if}
                  <button
                    class="recovery-btn"
                    type="button"
                    disabled={assistant.authChecking || assistant.loginInProgress}
                    onclick={() => assistant.recheckAuth()}
                  >
                    Re-check
                  </button>
                </div>
              </div>
            </div>
          {:else if showError}
            <div class="alert error">
              <span class="notice-icon">⚠</span>
              <span class="notice-text">{lastError}</span>
            </div>
          {/if}
        </div>
      {/if}

      <div
        class="composer-host"
        class:is-home={showEmpty}
        bind:this={composerSlotEl}
        onfocusin={onHostFocusIn}
        onfocusout={onHostFocusOut}
      >
        <Composer {tabId} hero={showEmpty && !engaged} onsubmit={handleSend} />
      </div>
    </div>
  {/if}

  {#if tabId && !showEmpty}
    <div class="scroll-fade-top" class:visible={scrolledTop} aria-hidden="true"></div>
  {/if}

  {#if tabId && !showEmpty && !stickToBottom}
    <button class="jump-latest" type="button" style="bottom: {composerH + 12}px" onclick={jumpToLatest} aria-label="Jump to latest message">
      <span class="jl-ic" aria-hidden="true"><ChevronDown size={13}/></span>
      Jump to latest
    </button>
  {/if}

  {#if tabId && !showEmpty && exchangeCount >= 2}
    <div class="turnrail" aria-label="Jump between turns">
      <button class="tr-chev" type="button" disabled={activeXch <= 0} onclick={() => stepXch(-1)} aria-label="Previous turn">
        <ChevronUp size={14} />
      </button>
      <div class="tr-ticks">
        {#each Array(exchangeCount) as _, i (i)}
          <button
            class="tr-tick"
            class:on={i === activeXch}
            type="button"
            onclick={() => scrollToXch(i)}
            aria-label={`Turn ${i + 1}`}
            aria-current={i === activeXch ? "true" : undefined}
          ></button>
        {/each}
      </div>
      <button class="tr-chev" type="button" disabled={activeXch >= exchangeCount - 1} onclick={() => stepXch(1)} aria-label="Next turn">
        <ChevronDown size={14} />
      </button>
    </div>
  {/if}

  {#if dragging}
    {#if assistant.splitActive}
      <div class="drop-zone full" class:hover={hoverHalf === "full"} aria-hidden="true">
        <span class="drop-label">{dragIsProject ? "Open here" : "Drop in pane"} {paneIdx + 1}</span>
      </div>
    {:else}
      <div class="drop-zone half left" class:hover={hoverHalf === "left"} aria-hidden="true">
        <span class="drop-label">{dragIsProject ? "Open" : "Split"} → left</span>
      </div>
      <div class="drop-zone half right" class:hover={hoverHalf === "right"} aria-hidden="true">
        <span class="drop-label">{dragIsProject ? "Open" : "Split"} → right</span>
      </div>
    {/if}
  {/if}
</div>
</div>

<style>
  .pane-shell {
    flex: 1 1 0;
    min-width: 320px;
    display: flex; flex-direction: row;
    min-height: 0;
  }
  .pane {
    flex: 1 1 0;
    min-width: 0;
    display: flex; flex-direction: column;
    min-height: 0;
    position: relative;
    border: 1px solid transparent;
    transition: border-color var(--dur-fast) ease-out, background var(--dur-fast) ease-out;
  }
  /* Split mode: each pane reads as a rounded card (spec .pane) — inset border,
     dimmed when not focused, accent ghost-ring on focus. Single pane stays bare. */
  .pane.split {
    border: 0;
    border-radius: 14px;
    overflow: hidden;
    margin: 6px 0 6px 6px;
    background: color-mix(in oklab, var(--bg-inset) 26%, transparent);
    box-shadow: inset 0 0 0 1px var(--border);
    transition: box-shadow var(--dur-base) var(--ease-soft), opacity var(--dur-base) var(--ease-soft);
    animation: paneIn var(--dur-base) var(--ease-page) backwards;
  }
  .pane.split:not(.focused) { opacity: 0.84; }

  .pane.split.focused { box-shadow: inset 0 0 0 1px var(--ghost-border); }
  .pane-shell:last-child .pane.split { margin-right: 6px; }
  @keyframes paneIn { from { transform: scale(0.985); } to { transform: none; } }
  @media (prefers-reduced-motion: reduce) { .pane.split { animation: none; } }
  /* Pane header — a slim, always-legible strip atop each pane in split mode.
     Replaces the old floating low-opacity chrome: shows the pane index, its
     conversation title (so a split is identifiable at a glance), the ctx chip
     and a close button. Focused pane gets an accent wash + brighter title. */
  .pane-head {
    position: relative;
    z-index: 4;
    flex-shrink: 0;
    display: flex; align-items: center; gap: 6px;
    height: 28px;
    padding: 0 6px 0 8px;
    background: color-mix(in oklch, var(--bg-elev-1) 55%, transparent);
    border-bottom: 1px solid var(--border);
    transition: background var(--dur-fast) ease-out, border-color var(--dur-fast) ease-out;
  }
  .pane-head.focused {
    background: color-mix(in oklab, var(--accent) 9%, var(--bg-elev-1));
    border-bottom-color: color-mix(in oklab, var(--accent) 40%, var(--border));
  }
  .pane-head-title {
    flex: 1; min-width: 0;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    font-size: var(--fs-xs);
    font-weight: 500;
    color: var(--fg-muted);
    transition: color var(--dur-fast) ease-out;
  }
  .pane-head.focused .pane-head-title { color: var(--fg); }
  .pane-label {
    flex-shrink: 0;
    display: inline-flex; align-items: center; justify-content: center;
    min-width: 16px; height: 16px;
    padding: 0 5px;
    border-radius: 999px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    color: var(--fg-muted);
    font-size: 10px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    line-height: 1;
  }
  .pane-ctx-chip {
    flex-shrink: 0;
    display: inline-flex; align-items: center; gap: 5px;
    height: 16px;
    padding: 0 6px;
    border-radius: 999px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    font-size: 10px;
    font-variant-numeric: tabular-nums;
    color: var(--fg-muted);
    line-height: 1;
  }
  /* Per-pane folder chip — matches the ctx-chip language but is interactive
     (each pane can run in its own project). Truncates the basename so a deep
     path can't blow out the head. */
  .pane-folder {
    flex-shrink: 1; min-width: 0;
    max-width: 130px;
    display: inline-flex; align-items: center; gap: 4px;
    height: 16px; padding: 0 7px 0 6px;
    border-radius: 999px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    color: var(--fg-muted);
    font-size: 10px; font-weight: 500; line-height: 1;
    cursor: pointer;
    transition: background var(--dur-fast), color var(--dur-fast), border-color var(--dur-fast);
  }
  .pane-folder :global(svg) { flex-shrink: 0; opacity: 0.8; }
  .pane-folder-name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pane-folder:hover {
    color: var(--fg);
    border-color: color-mix(in oklab, var(--accent) 35%, var(--border));
    background: color-mix(in oklab, var(--accent) 8%, var(--bg-elev-2));
  }
  /* Local-scratch mode — a faint accent tint so the badge reads as an active
     mode (clickable to switch to a real folder), not an error/empty state. */
  .pane-folder.local-mode {
    color: color-mix(in oklab, var(--accent) 55%, var(--fg-muted));
    border-color: color-mix(in oklab, var(--accent) 28%, var(--border));
    background: color-mix(in oklab, var(--accent) 7%, var(--bg-elev-2));
  }
  .pane-folder.local-mode :global(svg) { opacity: 0.95; }
  .pane-ctx-bar {
    width: 28px; height: 3px;
    background: var(--bg-elev-3, var(--border));
    border-radius: 2px;
    overflow: hidden;
    display: inline-block;
  }
  .pane-ctx-fill {
    display: block;
    height: 100%;
    background: var(--accent);
    transition: width var(--dur-base) ease-out;
  }
  .pane-ctx-pct { color: var(--fg); font-weight: 600; }
  .pane-cost { color: var(--fg-muted); }
  /* Per-pane working indicator — the concurrent-streaming cue. Pulse via
     box-shadow (never opacity: a throttled/backgrounded pane could freeze on
     an opacity:0 frame and read as "dead"). */
  .pane-live {
    flex-shrink: 0;
    display: inline-flex; align-items: center; gap: 5px;
    height: 16px; padding: 0 7px 0 6px;
    border-radius: 999px;
    background: var(--accent-soft);
    border: 1px solid color-mix(in oklab, var(--accent) 30%, var(--border));
    color: var(--accent);
    font-size: 10px; font-weight: 600; line-height: 1;
  }
  .pane-live-dot {
    width: 6px; height: 6px; border-radius: 50%;
    background: var(--accent);
    animation: pane-live-pulse var(--pulse-live, 1.6s) ease-out infinite;
  }
  @keyframes pane-live-pulse {
    0%, 100% { box-shadow: 0 0 0 0 color-mix(in oklab, var(--accent) 45%, transparent); }
    70% { box-shadow: 0 0 0 4px transparent; }
  }
  @media (prefers-reduced-motion: reduce) {
    .pane-live-dot { animation: none; }
  }
  .pane-ctx-chip[data-tone="yellow"] {
    border-color: color-mix(in oklab, var(--warn) 35%, var(--border));
  }
  .pane-ctx-chip[data-tone="yellow"] .pane-ctx-fill { background: var(--warn); }
  .pane-ctx-chip[data-tone="yellow"] .pane-ctx-pct { color: var(--warn); }
  .pane-ctx-chip[data-tone="red"] {
    border-color: color-mix(in oklab, var(--danger) 40%, var(--border));
  }
  .pane-ctx-chip[data-tone="red"] .pane-ctx-fill { background: var(--danger); }
  .pane-ctx-chip[data-tone="red"] .pane-ctx-pct { color: var(--danger); }

  .pane.focused .pane-label {
    color: var(--accent);
    border-color: color-mix(in oklab, var(--accent) 35%, var(--border));
    background: var(--accent-soft);
  }
  .pane-close {
    flex-shrink: 0;
    display: inline-flex; align-items: center; justify-content: center;
    width: 18px; height: 18px;
    padding: 0;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--fg-muted);
    cursor: pointer;
    transition: background var(--dur-fast), color var(--dur-fast), border-color var(--dur-fast);
  }
  .pane-close:hover {
    color: var(--danger);
    border-color: color-mix(in oklab, var(--danger) 35%, var(--border));
    background: color-mix(in oklab, var(--danger) 10%, transparent);
  }
  /* Focused pane — visible accent rail along the top edge plus a stronger
     border. Subtle in single-pane mode (no split visible); pronounced in
     split mode where multiple panes compete for attention. */
  .pane.split.focused {
    border-color: color-mix(in oklab, var(--accent) 60%, transparent);
    box-shadow: inset 0 2px 0 0 var(--accent);
  }
  .pane.split:not(.focused) {
    cursor: pointer;
    background: color-mix(in oklch, var(--bg) 94%, transparent);
  }
  .pane.split:not(.focused):hover {
    background: var(--bg);
    border-color: color-mix(in oklab, var(--accent) 22%, transparent);
  }
  /* ── Merged home ⇄ conversation column ──────────────────────────────────
     One flex column per pane. HOME centers its content (welcome + hero
     composer) as a unit and scrolls if it overflows; CONVERSATION lets the
     stream take the height with the composer docked at the bottom. The
     composer-host is shared across both so it FLIPs between the two. */
  .csurf-col {
    position: relative;
    z-index: 1;
    flex: 1; min-width: 0; min-height: 0;
    display: flex; flex-direction: column;
  }
  .csurf-col.is-home {
    overflow-y: auto;
    padding: 30px 32px;
    scrollbar-width: none;
  }
  /* Weighted centering via grow-spacers (54/46) — the welcome unit sits a
     touch below true center so the band under the composer stays smaller than
     the headroom above the greeting. Spacers collapse to zero when content
     overflows, so short windows just scroll (same safety `safe center` gave). */
  .csurf-col.is-home::before { content: ""; flex: 54 0 0; }
  .csurf-col.is-home::after { content: ""; flex: 46 0 0; }
  .csurf-col.is-home::-webkit-scrollbar { width: 0; height: 0; display: none; }
  .csurf-col.is-home > :global(*) {
    width: 100%; max-width: 680px;
    margin-left: auto; margin-right: auto;
    flex: none;
  }
  .csurf-col.is-home .composer-host { margin-top: 16px; }
  /* Engaged posture — welcome unmounted, composer sunk to the working edge.
     The FLIP in setEngaged animates the journey; this is the destination. */
  .csurf-col.is-home.engaged { justify-content: flex-end; }
  .csurf-col.is-home.engaged::before { flex: 1 0 0; }
  .csurf-col.is-home.engaged::after { flex: 0 0 0; }

  .composer-host {
    position: relative;
    z-index: 1;
    width: 100%;
    will-change: transform;
  }

  /* Pinned HUD stack (plan + agents) — owns the floating position both bars
     used to carry individually, so two live HUDs stack with a gap instead of
     overlapping. pointer-events pass through the empty stack area. */
  .hud-stack {
    position: absolute;
    top: 10px; left: 50%; transform: translateX(-50%);
    width: min(560px, calc(100% - 36px));
    z-index: 4;
    display: flex; flex-direction: column; gap: 8px;
    pointer-events: none;
  }
  .hud-stack > :global(*) { pointer-events: auto; }

  /* Conversation scroll region — replaces the old single `.scroll` wrapper for
     the message timeline. Keeps the hidden-scrollbar + flex-column behavior. */
  .stream {
    position: relative;
    z-index: 1;
    flex: 1; min-height: 0;
    overflow-y: auto;
    padding: 16px 18px 8px;
    scroll-padding-bottom: 28px;
    /* Follow-pin owns scroll position — native anchoring tugs against it when
       content above the viewport resizes (late highlight, output slides). */
    overflow-anchor: none;
    display: flex; flex-direction: column;
    scrollbar-width: none;
  }
  .stream::-webkit-scrollbar { width: 0; height: 0; display: none; }
  .stream::-webkit-scrollbar-button { display: none; }

  .scroll {
    position: relative;
    z-index: 1;
    flex: 1; min-height: 0;
    overflow-y: auto;
    padding: 16px 18px 28px;
    scroll-padding-bottom: 28px;
    display: flex; flex-direction: column;
    scrollbar-width: none;
  }
  .scroll::-webkit-scrollbar { width: 0; height: 0; display: none; }
  .scroll::-webkit-scrollbar-button { display: none; }
  /* Top fade — once the thread is scrolled down, content tucks under a soft
     shadow at the pane's top edge instead of hard-cutting at the scroll
     boundary. Fades in/out with scroll position (scrolledTop). */
  .scroll-fade-top {
    position: absolute;
    top: 0; left: 0; right: 0;
    height: 44px;
    pointer-events: none;
    z-index: 2;
    background: linear-gradient(
      to bottom,
      var(--bg) 0%,
      color-mix(in oklch, var(--bg) 55%, transparent) 50%,
      transparent 100%
    );
    opacity: 0;
    transition: opacity var(--dur-base) ease-out;
  }
  .scroll-fade-top.visible { opacity: 1; }
  @media (prefers-reduced-motion: reduce) {
    .scroll-fade-top { transition: none; }
  }
  .messages {
    display: flex; flex-direction: column;
    gap: 18px;
    max-width: var(--chat-col-max);
    width: 100%;
    margin: 0 auto;
  }
  /* Asymmetric rhythm groups each exchange: a question sits tight against its
     answer (14px base gap), while the break BEFORE a new user question opens
     up (+16px) so separate exchanges read as distinct blocks. Replaces the old
     uniform 28px that made every turn float equally. Rail-spans-turn still
     carries assistant grouping; no divider line. */
  .messages > :global(.bubble[data-role="user"]:not(:first-child)) {
    margin-top: 18px;
  }

  .pane-empty {
    flex: 1;
    display: flex; align-items: center; justify-content: center;
    padding: 24px;
  }
  .pane-empty-card {
    display: flex; flex-direction: column; align-items: center;
    gap: 9px;
    padding: 22px 22px 18px;
    border: 1px solid var(--border);
    border-radius: var(--r-card, var(--radius-lg));
    background: color-mix(in oklch, var(--bg-elev-1) 70%, transparent);
    box-shadow: 0 14px 38px -18px rgba(0, 0, 0, 0.55);
    max-width: 340px; width: 100%;
    animation: pane-empty-in 320ms cubic-bezier(0.22, 1, 0.36, 1) both;
  }
  @media (prefers-reduced-motion: reduce) {
    .pane-empty-card { animation: none; }
  }
  @keyframes pane-empty-in {
    from { opacity: 0; transform: translateY(6px) scale(0.985); }
    to   { opacity: 1; transform: none; }
  }
  .pane-empty-mark {
    width: 44px; height: 44px;
    border-radius: var(--radius-xl, 14px);
    display: grid; place-items: center;
    background: var(--accent-soft); color: var(--accent);
    margin-bottom: 1px;
  }
  .pane-empty-title {
    font-size: var(--fs-md);
    color: var(--fg);
    font-weight: 650;
    text-align: center;
    letter-spacing: -0.01em;
  }
  .pane-empty-hint {
    font-size: var(--fs-sm);
    color: var(--fg-muted);
    text-align: center;
    line-height: 1.45;
    max-width: 240px;
  }
  .pane-empty-actions {
    display: flex; gap: 8px;
    justify-content: center;
    margin-top: 6px;
  }
  .pane-empty-recent {
    display: flex; flex-direction: column;
    gap: 3px;
    margin-top: 12px;
    padding-top: 14px;
    border-top: 1px solid var(--border);
    width: 100%;
  }
  .pane-empty-recent-label {
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--fg-faint);
    font-weight: 600;
    margin-bottom: 3px;
    padding: 0 4px;
  }
  .pane-empty-recent-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 9px;
    border-radius: 9px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--fg);
    font: inherit;
    font-size: var(--fs-sm);
    cursor: pointer;
    text-align: left;
    transition: background var(--dur-fast) ease, border-color var(--dur-fast) ease;
  }
  .pane-empty-recent-row:hover {
    background: var(--surface-hover);
    border-color: var(--border);
  }
  .pane-empty-recent-title {
    flex: 1; min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pane-empty-recent-meta {
    flex-shrink: 0;
    font-size: var(--fs-xs);
    color: var(--fg-faint);
    font-family: var(--font-mono, monospace);
  }
  :global(.pane-empty-recent-chev) {
    flex-shrink: 0;
    color: var(--fg-faint);
    opacity: 0;
    transform: translateX(-3px);
    transition: opacity var(--dur-fast) ease, transform var(--dur-fast) ease, color var(--dur-fast) ease;
  }
  .pane-empty-recent-row:hover :global(.pane-empty-recent-chev) {
    opacity: 1;
    transform: none;
    color: var(--accent);
  }

  /* Compact circular "scroll to latest" affordance, parked at the bottom-right
     of the message area just clear of the composer's top edge — the chat-app
     convention (Slack/Discord/ChatGPT). Icon-only; the chevron gently bobs to
     signal "new below". Replaced the centered text-pill, which read as bulky. */
  .jump-latest {
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    display: inline-flex; align-items: center; gap: 7px;
    height: 32px; padding: 0 14px 0 12px;
    border-radius: 999px;
    background: color-mix(in oklch, var(--surface) 82%, transparent);
    backdrop-filter: blur(16px) saturate(140%);
    -webkit-backdrop-filter: blur(16px) saturate(140%);
    border: 1px solid var(--border-strong);
    color: var(--fg-2);
    font-size: 12px; font-weight: 550;
    cursor: pointer;
    box-shadow: var(--shadow-float);
    z-index: 3;
    animation: jump-in var(--dur-base) cubic-bezier(0.22, 1, 0.36, 1);
    transition: color var(--dur-fast) ease, border-color var(--dur-fast) ease;
  }
  .jump-latest:hover { color: var(--fg); border-color: var(--accent); }
  .jl-ic {
    display: grid; place-items: center;
    width: 18px; height: 18px; border-radius: 50%;
    background: var(--accent); color: var(--accent-fg);
  }
  @keyframes jump-in {
    from { opacity: 0; transform: translateX(-50%) translateY(8px); }
    to   { opacity: 1; transform: translateX(-50%) translateY(0); }
  }
  @media (prefers-reduced-motion: reduce) {
    .jump-latest { animation: none; }
  }
  .turnrail {
    position: absolute; right: 9px; top: 50%; transform: translateY(-50%); z-index: 4;
    display: flex; flex-direction: column; align-items: flex-end; gap: 5px; padding: 5px 3px;
    opacity: 0.45; transition: opacity var(--dur-base);
  }
  .turnrail:hover { opacity: 1; }
  .tr-chev {
    width: 24px; height: 18px; display: grid; place-items: center;
    background: none; border: 0; cursor: pointer;
    color: var(--fg-faint); border-radius: 6px;
    transition: color var(--dur-fast), background var(--dur-fast);
  }
  .tr-chev:hover:not(:disabled) { color: var(--fg-2); background: var(--surface-hover); }
  .tr-chev:disabled { opacity: 0.3; cursor: default; }
  .tr-ticks { display: flex; flex-direction: column; align-items: flex-end; gap: 6px; padding: 3px 6px; }
  .tr-tick {
    position: relative; height: 11px; display: flex; align-items: center; justify-content: flex-end;
    background: none; border: 0; padding: 0; cursor: pointer;
  }
  .tr-tick::after {
    content: ""; height: 2px; width: 12px; border-radius: 2px; background: var(--fg-faint); opacity: 0.55;
    transition: width var(--dur-fast), background var(--dur-fast), opacity var(--dur-fast);
  }
  .tr-tick:hover::after { width: 18px; opacity: 1; background: var(--fg-muted); }
  .tr-tick.on::after { width: 20px; background: var(--accent); opacity: 1; }

  .alerts {
    position: relative;
    z-index: 1;
    flex-shrink: 0;
    display: flex; flex-direction: column;
    gap: 6px;
    padding: 6px 18px 0;
    max-width: var(--chat-col-max);
    width: 100%;
    margin: 0 auto;
  }
  .alert {
    display: flex; align-items: flex-start; gap: 9px;
    width: 100%;
    padding: 8px 12px;
    border-radius: 8px;
    font: inherit;
    font-size: var(--fs-sm);
    text-align: left;
    line-height: 1.4;
    animation: enter var(--dur-base) cubic-bezier(0.22, 1, 0.36, 1);
  }
  .alert.error {
    background: var(--danger-soft);
    border: 1px solid color-mix(in oklab, var(--danger) 35%, transparent);
    color: oklch(0.92 0.05 22);
  }
  .alert.error .notice-icon { color: var(--danger); }
  .alert.notice {
    background: color-mix(in oklab, var(--accent) 10%, var(--surface));
    border: 1px solid color-mix(in oklab, var(--accent) 30%, var(--border));
    color: var(--fg-2);
    cursor: pointer;
    transition: background var(--dur-fast) ease-out, border-color var(--dur-fast) ease-out;
  }
  .alert.notice:hover {
    background: color-mix(in oklab, var(--accent) 14%, var(--surface));
    border-color: color-mix(in oklab, var(--accent) 50%, var(--border));
  }
  .notice-icon {
    color: var(--accent);
    font-weight: 700;
    line-height: 1.4;
    flex-shrink: 0;
  }
  .notice-text { flex: 1; line-height: 1.45; }
  /* Auth recovery: error text stacked over an action row. */
  .alert.error.recovery { align-items: flex-start; }
  .recovery-body { flex: 1; display: flex; flex-direction: column; gap: 8px; min-width: 0; }
  .recovery-actions { display: flex; gap: 8px; flex-wrap: wrap; }
  .recovery-btn {
    font: inherit;
    font-size: var(--fs-sm);
    font-weight: 600;
    padding: 4px 12px;
    border-radius: 6px;
    cursor: pointer;
    border: 1px solid color-mix(in oklab, var(--danger) 40%, transparent);
    background: color-mix(in oklab, var(--danger) 12%, var(--surface));
    color: oklch(0.92 0.05 22);
    transition: background var(--dur-fast) ease-out, border-color var(--dur-fast) ease-out, opacity var(--dur-fast) ease-out;
  }
  .recovery-btn:hover:not(:disabled) {
    background: color-mix(in oklab, var(--danger) 20%, var(--surface));
    border-color: color-mix(in oklab, var(--danger) 60%, transparent);
  }
  .recovery-btn.primary {
    background: var(--danger);
    border-color: var(--danger);
    color: oklch(0.99 0.01 22);
  }
  .recovery-btn.primary:hover:not(:disabled) {
    background: color-mix(in oklab, var(--danger) 88%, white);
  }
  .recovery-btn:disabled { opacity: 0.6; cursor: default; }
  @media (prefers-reduced-motion: reduce) {
    .alert { animation: none; }
  }

  /* Drag-drop overlay zones — fade in while a tab is being dragged from the
     ChatTabsBar. `.full` covers the whole pane (split mode); `.half left`/
     `.half right` carve the pane in two so the user can pick which side to
     split into (single-pane mode). */
  .drop-zone {
    position: absolute;
    top: 0; bottom: 0;
    display: flex; align-items: center; justify-content: center;
    z-index: 5;
    /* Visual only — drop handling lives on the .pane parent so dragover is
       continuously preventDefault'd across all child crossings. */
    pointer-events: none;
    background: color-mix(in oklab, var(--accent) 8%, transparent);
    border: 1px dashed color-mix(in oklab, var(--accent) 40%, transparent);
    color: color-mix(in oklab, var(--accent) 80%, var(--fg));
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: 0.02em;
    transition: background var(--dur-fast) ease-out, border-color var(--dur-fast) ease-out;
    animation: drop-in var(--dur-fast) cubic-bezier(0.22, 1, 0.36, 1);
  }
  .drop-zone.full { left: 0; right: 0; }
  .drop-zone.half.left { left: 0; right: 50%; }
  .drop-zone.half.right { left: 50%; right: 0; }
  .drop-zone.hover {
    background: color-mix(in oklab, var(--accent) 20%, transparent);
    border-color: var(--accent);
    color: var(--accent);
  }
  .drop-label {
    padding: 6px 14px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    box-shadow: 0 6px 18px oklch(0 0 0 / 0.32);
    /* Critical: children must not capture pointer events or they'll fire
       dragleave on the parent zone every time the cursor crosses the pill,
       flickering hover state + losing the dragover preventDefault chain. */
    pointer-events: none;
  }
  @keyframes drop-in {
    from { opacity: 0; }
    to   { opacity: 1; }
  }
  @media (prefers-reduced-motion: reduce) {
    .drop-zone { animation: none; }
  }
</style>

