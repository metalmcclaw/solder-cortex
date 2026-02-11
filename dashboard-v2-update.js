// Dashboard Update Script - Convert from wallet-focused to market-wide data
// Replace the JavaScript section in dashboard/index.html with these functions

// New Market-Wide Data Functions
async function loadMarketOverview() {
  try {
    const response = await fetch('/api/v2/market/overview');
    const data = await response.json();
    
    // Update DeFi metrics
    updateDeFiMetrics(data.defi);
    
    // Update prediction market metrics  
    updatePredictionMetrics(data.prediction_markets);
    
    // Update market sentiment
    updateMarketSentiment(data.sentiment);
    
    console.log('Market overview loaded:', data);
    return data;
  } catch (error) {
    console.error('Failed to load market overview:', error);
    showError('Failed to load market data');
  }
}

async function loadConvictionSignals() {
  try {
    const response = await fetch('/api/v2/market/conviction');
    const data = await response.json();
    
    updateConvictionAnalysis(data);
    console.log('Conviction signals loaded:', data);
    return data;
  } catch (error) {
    console.error('Failed to load conviction signals:', error);
  }
}

function updateDeFiMetrics(defiData) {
  // Update total TVL
  const tvlElement = document.querySelector('[data-metric="total-tvl"]');
  if (tvlElement) {
    tvlElement.textContent = formatCurrency(defiData.total_tvl);
  }
  
  // Update total volume
  const volumeElement = document.querySelector('[data-metric="total-volume"]');
  if (volumeElement) {
    volumeElement.textContent = formatCurrency(defiData.total_volume_24h);
  }
  
  // Update protocol breakdown
  const protocolContainer = document.querySelector('[data-section="protocol-breakdown"]');
  if (protocolContainer) {
    protocolContainer.innerHTML = '';
    
    Object.entries(defiData.protocol_breakdown).forEach(([protocol, stats]) => {
      const protocolElement = createProtocolCard(protocol, stats);
      protocolContainer.appendChild(protocolElement);
    });
  }
  
  // Update trending tokens
  updateTrendingTokens(defiData.trending_tokens);
}

function updatePredictionMetrics(predictionData) {
  // Update market count
  const marketCountElement = document.querySelector('[data-metric="market-count"]');
  if (marketCountElement) {
    marketCountElement.textContent = predictionData.total_markets.toLocaleString();
  }
  
  // Update prediction volume
  const predVolumeElement = document.querySelector('[data-metric="prediction-volume"]');
  if (predVolumeElement) {
    predVolumeElement.textContent = formatCurrency(predictionData.total_volume_24h);
  }
  
  // Update featured markets
  updateFeaturedMarkets(predictionData.featured_markets);
  
  // Update trending topics
  updateTrendingTopics(predictionData.trending_topics);
}

function updateMarketSentiment(sentimentData) {
  const sentimentContainer = document.querySelector('[data-section="market-sentiment"]');
  if (!sentimentContainer) return;
  
  // Overall sentiment score
  const overallScore = sentimentData.overall_score;
  const sentimentClass = overallScore > 0.2 ? 'bullish' : overallScore < -0.2 ? 'bearish' : 'neutral';
  
  sentimentContainer.innerHTML = `
    <div class="sentiment-overview ${sentimentClass}">
      <div class="sentiment-score">
        <span class="score-value">${(overallScore * 100).toFixed(1)}%</span>
        <span class="score-label">${getSentimentLabel(overallScore)}</span>
      </div>
      <div class="confidence">
        Confidence: ${(sentimentData.confidence * 100).toFixed(0)}%
      </div>
    </div>
    
    <div class="sentiment-breakdown">
      <div class="breakdown-item">
        <span class="label">DeFi Sentiment:</span>
        <span class="value ${getSentimentClass(sentimentData.defi_sentiment)}">
          ${(sentimentData.defi_sentiment * 100).toFixed(1)}%
        </span>
      </div>
      <div class="breakdown-item">
        <span class="label">Prediction Markets:</span>
        <span class="value ${getSentimentClass(sentimentData.prediction_sentiment)}">
          ${(sentimentData.prediction_sentiment * 100).toFixed(1)}%
        </span>
      </div>
    </div>
    
    <div class="sentiment-drivers">
      <h4>Key Drivers:</h4>
      ${sentimentData.sentiment_drivers.map(driver => `
        <div class="driver-item ${getSentimentClass(driver.impact)}">
          <span class="factor">${driver.factor}</span>
          <span class="impact">${(driver.impact * 100).toFixed(1)}%</span>
          <div class="description">${driver.description}</div>
        </div>
      `).join('')}
    </div>
  `;
}

function updateConvictionAnalysis(convictionData) {
  const convictionContainer = document.querySelector('[data-section="conviction-analysis"]');
  if (!convictionContainer) return;
  
  const { signals, summary } = convictionData;
  
  convictionContainer.innerHTML = `
    <div class="conviction-summary">
      <div class="overall-conviction ${getConvictionClass(summary.overall_conviction)}">
        <span class="conviction-score">${(summary.overall_conviction * 100).toFixed(1)}%</span>
        <span class="conviction-label">Overall Conviction</span>
      </div>
      
      <div class="signal-counts">
        <div class="signal-count bullish">
          <span class="count">${summary.bull_signals}</span>
          <span class="label">Bull Signals</span>
        </div>
        <div class="signal-count bearish">
          <span class="count">${summary.bear_signals}</span>
          <span class="label">Bear Signals</span>
        </div>
        <div class="signal-count conflicting">
          <span class="count">${summary.conflicting_signals}</span>
          <span class="label">Conflicting</span>
        </div>
      </div>
      
      <div class="data-completeness">
        Data Completeness: ${(summary.data_completeness * 100).toFixed(0)}%
        <div class="progress-bar">
          <div class="progress-fill" style="width: ${summary.data_completeness * 100}%"></div>
        </div>
      </div>
    </div>
    
    <div class="conviction-signals">
      <h4>Active Signals:</h4>
      ${signals.map(signal => `
        <div class="signal-card ${signal.signal_type}">
          <div class="signal-header">
            <span class="signal-type">${signal.signal_type.replace(/_/g, ' ').toUpperCase()}</span>
            <span class="signal-strength">${(signal.strength * 100).toFixed(0)}%</span>
          </div>
          <div class="signal-description">${signal.description}</div>
          <div class="signal-confidence">
            Confidence: ${(signal.confidence * 100).toFixed(0)}%
          </div>
          <div class="signal-timestamp">
            ${new Date(signal.timestamp).toLocaleString()}
          </div>
        </div>
      `).join('')}
    </div>
  `;
}

// Helper Functions
function createProtocolCard(protocol, stats) {
  const card = document.createElement('div');
  card.className = 'protocol-card';
  card.innerHTML = `
    <div class="protocol-name">${protocol.toUpperCase()}</div>
    <div class="protocol-stats">
      <div class="stat">
        <span class="label">TVL:</span>
        <span class="value">${formatCurrency(stats.tvl)}</span>
      </div>
      <div class="stat">
        <span class="label">24h Vol:</span>
        <span class="value">${formatCurrency(stats.volume_24h)}</span>
      </div>
      <div class="stat">
        <span class="label">Market Share:</span>
        <span class="value">${stats.market_share.toFixed(1)}%</span>
      </div>
    </div>
  `;
  return card;
}

function updateTrendingTokens(tokens) {
  const container = document.querySelector('[data-section="trending-tokens"]');
  if (!container) return;
  
  container.innerHTML = tokens.slice(0, 5).map(token => `
    <div class="token-item">
      <div class="token-symbol">${token.symbol}</div>
      <div class="token-price">$${token.price_usd.toFixed(4)}</div>
      <div class="token-change ${token.price_change_24h >= 0 ? 'positive' : 'negative'}">
        ${token.price_change_24h >= 0 ? '+' : ''}${token.price_change_24h.toFixed(2)}%
      </div>
      <div class="defi-activity">
        Activity: ${token.defi_activity_score.toFixed(2)}
      </div>
    </div>
  `).join('');
}

function updateFeaturedMarkets(markets) {
  const container = document.querySelector('[data-section="featured-markets"]');
  if (!container) return;
  
  container.innerHTML = markets.slice(0, 3).map(market => `
    <div class="market-card">
      <div class="market-platform">${market.platform.toUpperCase()}</div>
      <div class="market-question">${market.question}</div>
      <div class="market-prices">
        <span class="yes-price">YES: ${(market.yes_price * 100).toFixed(1)}%</span>
        <span class="no-price">NO: ${(market.no_price * 100).toFixed(1)}%</span>
      </div>
      <div class="market-volume">
        24h Volume: ${formatCurrency(market.volume_24h)}
      </div>
      <div class="conviction-score">
        Conviction: ${(market.conviction_score * 100).toFixed(0)}%
      </div>
    </div>
  `).join('');
}

function updateTrendingTopics(topics) {
  const container = document.querySelector('[data-section="trending-topics"]');
  if (!container) return;
  
  container.innerHTML = topics.slice(0, 8).map(topic => `
    <span class="topic-tag">${topic}</span>
  `).join('');
}

// Utility Functions
function formatCurrency(value) {
  if (value >= 1e9) {
    return `$${(value / 1e9).toFixed(2)}B`;
  } else if (value >= 1e6) {
    return `$${(value / 1e6).toFixed(2)}M`;
  } else if (value >= 1e3) {
    return `$${(value / 1e3).toFixed(1)}K`;
  } else {
    return `$${value.toFixed(2)}`;
  }
}

function getSentimentLabel(score) {
  if (score > 0.3) return 'Very Bullish';
  if (score > 0.1) return 'Bullish';
  if (score > -0.1) return 'Neutral';
  if (score > -0.3) return 'Bearish';
  return 'Very Bearish';
}

function getSentimentClass(score) {
  if (score > 0.1) return 'positive';
  if (score < -0.1) return 'negative';
  return 'neutral';
}

function getConvictionClass(score) {
  if (score > 0.2) return 'high-conviction bullish';
  if (score < -0.2) return 'high-conviction bearish';
  return 'low-conviction';
}

function showError(message) {
  console.error(message);
  // Add error display logic
}

// Initialize market-wide dashboard
async function initializeMarketDashboard() {
  console.log('Initializing market-wide dashboard...');
  
  try {
    await loadMarketOverview();
    await loadConvictionSignals();
    
    // Set up auto-refresh
    setInterval(() => {
      loadMarketOverview();
    }, 5 * 60 * 1000); // 5 minutes
    
    setInterval(() => {
      loadConvictionSignals();
    }, 10 * 60 * 1000); // 10 minutes
    
    console.log('Market dashboard initialized successfully');
  } catch (error) {
    console.error('Failed to initialize dashboard:', error);
  }
}

// Start the market dashboard when page loads
document.addEventListener('DOMContentLoaded', initializeMarketDashboard);