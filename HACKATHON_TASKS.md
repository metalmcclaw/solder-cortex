# Solder Cortex - Colosseum Hackathon Tasks

**Deadline:** Feb 12, 2026 (3 days remaining)
**Track:** AI x DeFi

## P0 - Critical (Must Have for Submission)

### Infrastructure
- [x] Deploy production stack on VPS (Docker Compose)
- [x] Configure PostgreSQL + ClickHouse
- [x] Set up Caddy reverse proxy
- [x] Add Helius + LysLabs API keys
- [x] Enable Cortex API server ✅
- [x] Verify live data flow works end-to-end ✅
- [ ] Add domain + HTTPS (optional but nice)

### Demo
- [x] Generate pitch voiceover audio ✅
- [x] Generate technical demo voiceover audio ✅
- [x] Generate video clips with Veo 3 AI ✅ (8 clips)
- [x] Combine audio + video into final files ✅
- [x] **Pitch video complete:** `solder_cortex_pitch.mp4` (1:55, 28.5MB)
- [x] **Technical demo complete:** `solder_cortex_technical_demo.mp4` (2:00, 43.5MB)
- [ ] Upload videos to YouTube/Loom
- [ ] Add video links to Colosseum submission

### Submission
- [ ] Update GitHub repo with final code
- [ ] Write submission description
- [ ] Submit to Colosseum before deadline

## P1 - High Priority (Improves Chances)

### Features
- [ ] Add more prediction market sources (Kalshi data)
- [ ] Improve conviction scoring algorithm
- [ ] Add historical conviction tracking
- [ ] Add wallet comparison tool

### Marketing
- [x] Post announcement on Moltbook
- [x] Engage with comments
- [ ] Post on Colosseum forum (requires manual auth)
- [ ] Reach out to integration partners

### Documentation
- [x] Update README with live demo URL ✅
- [x] Create submission description (COLOSSEUM_SUBMISSION.md) ✅
- [ ] Add API documentation
- [x] Create Claude Desktop config example (in README)

## P2 - Nice to Have

- [ ] Add Telegram/Discord bot interface
- [ ] Add email alerts for conviction changes
- [ ] Add leaderboard of informed traders
- [ ] Mobile-friendly dashboard

## Integration Opportunities

Potential partners to reach out to:
- [ ] Solana Agent Kit - AI agent framework
- [ ] AgentDex - Agent trading platform
- [ ] NEXUS - AI infrastructure
- [ ] SolanaDD - Due diligence tools

## Daily Log

### Feb 9, 2026
- Deployed production Docker stack
- Configured API keys (Helius, LysLabs)
- Posted Moltbook announcement (2 upvotes, 7+ comments)
- Built & deployed Cortex API server ✅
- Live data working - tested with Jupiter wallet indexing (400+ txns → 847 txns now)
- Full 5-container stack running: Caddy, Web, Cortex, PostgreSQL, ClickHouse
- Generated pitch voiceover audio (677KB)
- Generated technical demo voiceover audio (701KB)
- Updated README with live demo URL
- Created COLOSSEUM_SUBMISSION.md with full submission details
- Pushed updates to GitHub
- **Unblocked Colosseum auth** - retrieved API keys from hackathon agent repo
- **Forum engagement activated** - 4 posts, 12+ comments, all with vote requests
- **8 integration opportunities** identified and documented
- **Live demo confirmed** - VPS running with real-time data (22K+ messages processed)

### Feb 8, 2026
- Fixed server crash on MCP spawn error
- Created web dashboard
- Set up WebSocket bridge

## Notes

- Demo URL: http://76.13.193.103/
- Moltbook post: https://moltbook.com/post/f262a260-33b4-4d0e-901b-52c4e3cfa09a
- Need Notion access to sync tasks (credentials missing)
