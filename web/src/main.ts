import { LitElement, css, html, nothing } from 'lit';
import { customElement, state } from 'lit/decorators.js';

type EditionItem = {
  id: number;
  url: string;
  title: string;
  source_type: string;
  published_at: string;
  summary?: string;
  key_points: string[];
  score?: number;
  reason?: string;
};

type Edition = {
  id: number;
  target_date: string;
  timezone: string;
  daily_limit: number;
  items: EditionItem[];
};

@customElement('rastraq-app')
class RastraqApp extends LitElement {
  @state() private edition?: Edition;
  @state() private selectedDate = '';
  @state() private loading = true;
  @state() private error = '';
  @state() private feedback = new Map<number, string>();

  connectedCallback() {
    super.connectedCallback();
    void this.loadToday();
  }

  render() {
    return html`
      <main>
        <header class="topbar">
          <div>
            <p class="eyebrow">Rastraq</p>
            <h1>Daily Radar</h1>
          </div>
          <form class="date-form" @submit=${this.loadDate}>
            <input
              type="date"
              .value=${this.selectedDate}
              @input=${(event: Event) => {
                this.selectedDate = (event.target as HTMLInputElement).value;
              }}
            />
            <button type="submit">表示</button>
            <button type="button" @click=${this.loadToday}>今日</button>
          </form>
        </header>

        ${this.loading ? html`<section class="state">Loading</section>` : nothing}
        ${this.error ? html`<section class="state error">${this.error}</section>` : nothing}
        ${this.edition ? this.renderEdition(this.edition) : nothing}
      </main>
    `;
  }

  private renderEdition(edition: Edition) {
    return html`
      <section class="edition-head">
        <div>
          <p>${edition.timezone}</p>
          <h2>${edition.target_date}</h2>
        </div>
        <span>${edition.items.length} / ${edition.daily_limit}</span>
      </section>
      <section class="items">
        ${edition.items.map((item, index) => this.renderItem(item, index + 1))}
      </section>
    `;
  }

  private renderItem(item: EditionItem, rank: number) {
    return html`
      <article class="item">
        <div class="rank">${rank}</div>
        <div class="body">
          <div class="meta">
            <span>${item.source_type}</span>
            <time>${new Date(item.published_at).toLocaleString()}</time>
          </div>
          <h3><a href=${item.url} target="_blank" rel="noreferrer" @click=${() => this.record(item, 'clicked')}>${item.title}</a></h3>
          <p class="summary">${item.summary ?? ''}</p>
          ${item.reason ? html`<p class="reason">${item.reason}</p>` : nothing}
          <div class="actions">
            <button @click=${() => this.record(item, 'interested')}>興味あり</button>
            <button @click=${() => this.record(item, 'not_interested')}>興味なし</button>
            <button @click=${() => this.record(item, 'saved')}>保存</button>
            <button @click=${() => this.record(item, 'read')}>既読</button>
            <span>${this.feedback.get(item.id) ?? ''}</span>
          </div>
        </div>
      </article>
    `;
  }

  private async loadToday() {
    await this.load('/api/editions/today');
  }

  private loadDate = async (event: Event) => {
    event.preventDefault();
    if (!this.selectedDate) return;
    await this.load(`/api/editions?date=${this.selectedDate}`);
  };

  private async load(path: string) {
    this.loading = true;
    this.error = '';
    try {
      const response = await fetch(path);
      if (!response.ok) throw new Error(await response.text());
      this.edition = await response.json();
      this.selectedDate = this.edition?.target_date ?? this.selectedDate;
    } catch (error) {
      this.edition = undefined;
      this.error = error instanceof Error ? error.message : String(error);
    } finally {
      this.loading = false;
    }
  }

  private async record(item: EditionItem, eventType: string) {
    await fetch('/api/feedback', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({
        item_id: item.id,
        event_type: eventType,
        payload: { surface: 'daily-card', target_date: this.edition?.target_date },
      }),
    });
    this.feedback = new Map(this.feedback).set(item.id, eventType);
  }

  static styles = css`
    :host {
      color: #18201c;
      font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    }

    main {
      max-width: 1040px;
      margin: 0 auto;
      padding: 32px 20px 56px;
    }

    .topbar,
    .edition-head {
      align-items: end;
      display: flex;
      justify-content: space-between;
      gap: 16px;
      border-bottom: 1px solid #d6ddd4;
      padding-bottom: 18px;
    }

    .eyebrow,
    .edition-head p,
    .meta,
    .reason {
      color: #5a665e;
      font-size: 0.84rem;
      margin: 0;
    }

    h1,
    h2,
    h3 {
      letter-spacing: 0;
      margin: 0;
    }

    h1 {
      font-size: 2.5rem;
    }

    h2 {
      font-size: 1.8rem;
    }

    .date-form,
    .actions {
      display: flex;
      gap: 8px;
      flex-wrap: wrap;
    }

    input,
    button {
      border: 1px solid #b8c3bc;
      border-radius: 6px;
      background: #ffffff;
      color: #18201c;
      font: inherit;
      min-height: 36px;
      padding: 6px 10px;
    }

    button {
      cursor: pointer;
    }

    .state {
      margin: 24px 0;
      padding: 14px 0;
      border-bottom: 1px solid #d6ddd4;
    }

    .error {
      color: #9a2a18;
    }

    .edition-head {
      margin-top: 28px;
    }

    .edition-head span {
      border: 1px solid #b8c3bc;
      border-radius: 999px;
      padding: 6px 10px;
    }

    .items {
      display: grid;
      gap: 14px;
      margin-top: 20px;
    }

    .item {
      display: grid;
      grid-template-columns: 42px 1fr;
      gap: 14px;
      border: 1px solid #d6ddd4;
      border-radius: 8px;
      padding: 16px;
      background: #fbfcfa;
    }

    .rank {
      align-items: center;
      background: #1f3d2b;
      border-radius: 6px;
      color: white;
      display: flex;
      font-weight: 700;
      height: 42px;
      justify-content: center;
      width: 42px;
    }

    .body {
      min-width: 0;
    }

    .meta {
      display: flex;
      gap: 12px;
      flex-wrap: wrap;
    }

    a {
      color: #123c69;
      text-decoration-thickness: 1px;
      text-underline-offset: 3px;
    }

    h3 {
      font-size: 1.2rem;
      margin-top: 4px;
    }

    .summary {
      line-height: 1.6;
      margin: 10px 0;
    }

    .actions span {
      align-self: center;
      color: #3a6b46;
      min-width: 84px;
    }

    @media (max-width: 720px) {
      .topbar,
      .edition-head {
        align-items: start;
        flex-direction: column;
      }

      h1 {
        font-size: 2rem;
      }

      .item {
        grid-template-columns: 1fr;
      }
    }
  `;
}
