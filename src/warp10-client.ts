import axios, { AxiosInstance } from 'axios';

export interface Warp10Config {
  endpoint: string;
  token?: string;
  readToken?: string;
  writeToken?: string;
}

export interface FetchParams {
  selector: string;
  labels?: Record<string, string>;
  start?: string | number;
  end?: string | number;
  count?: number;
}

export interface UpdateParams {
  className: string;
  labels?: Record<string, string>;
  dataPoints: Array<{
    timestamp: number;
    latitude?: number;
    longitude?: number;
    elevation?: number;
    value: any;
  }>;
}

export interface DeleteParams {
  selector: string;
  labels?: Record<string, string>;
  start?: string | number;
  end?: string | number;
}

export interface FindParams {
  selector: string;
  labels?: Record<string, string>;
}

export interface MetaParams {
  selector: string;
  labels?: Record<string, string>;
  attributes?: Record<string, string>;
}

export interface LabelValuesParams {
  selector: string;
  labelName: string;
  otherLabels?: Record<string, string>;
}

export interface PredictParams {
  selector: string;
  labels?: Record<string, string>;
  method: 'holt_winters' | 'arima' | 'ses' | 'linear_regression' | 'polynomial' | 'fft_forecast' | 'lstm' | 'auto';
  historyPeriod: string;
  predictionHorizon: number;
  bucketSpan?: string;
  options?: {
    // Holt-Winters
    alpha?: number;
    beta?: number;
    gamma?: number;
    seasonality?: number;
    // ARIMA
    p?: number;
    d?: number;
    q?: number;
    // Polynomial
    degree?: number;
    // LSTM
    epochs?: number;
    hiddenUnits?: number;
  };
  includeConfidence?: boolean;
  confidenceLevel?: number;
}

export class Warp10Client {
  private axios: AxiosInstance;
  private config: Warp10Config;

  constructor(config: Warp10Config) {
    this.config = config;
    this.axios = axios.create({
      baseURL: config.endpoint,
      headers: this.getHeaders()
    });
  }

  private getHeaders(operation?: 'read' | 'write') {
    const headers: Record<string, string> = {};

    if (operation === 'read' && this.config.readToken) {
      headers['X-Warp10-Token'] = this.config.readToken;
    } else if (operation === 'write' && this.config.writeToken) {
      headers['X-Warp10-Token'] = this.config.writeToken;
    } else if (this.config.token) {
      headers['X-Warp10-Token'] = this.config.token;
    }

    return headers;
  }

  async fetch(params: FetchParams): Promise<any> {
    const script = this.buildFetchScript(params);
    return this.exec(script, 'read');
  }

  async update(params: UpdateParams): Promise<string> {
    const gts = this.buildGTSInput(params);

    const response = await this.axios.post('/update', gts, {
      headers: {
        ...this.getHeaders('write'),
        'Content-Type': 'text/plain'
      }
    });

    return response.data;
  }

  async delete(params: DeleteParams): Promise<string> {
    const deleteParams = new URLSearchParams();
    deleteParams.append('selector', params.selector);

    if (params.labels) {
      deleteParams.append('labels', this.formatLabels(params.labels));
    }

    if (params.start !== undefined) {
      deleteParams.append('start', String(params.start));
    }

    if (params.end !== undefined) {
      deleteParams.append('end', String(params.end));
    }

    const response = await this.axios.get(`/delete?${deleteParams.toString()}`, {
      headers: this.getHeaders('write')
    });

    return response.data;
  }

  async exec(script: string, operation?: 'read' | 'write'): Promise<any> {
    const response = await this.axios.post('/exec', script, {
      headers: {
        ...this.getHeaders(operation),
        'Content-Type': 'text/plain'
      }
    });

    return response.data;
  }

  async find(params: FindParams): Promise<any> {
    const script = this.buildFindScript(params);
    return this.exec(script, 'read');
  }

  async meta(params: MetaParams): Promise<any> {
    const script = this.buildMetaScript(params);
    return this.exec(script, params.attributes ? 'write' : 'read');
  }

  async getLabelValues(params: LabelValuesParams): Promise<string[]> {
    const script = this.buildLabelValuesScript(params);
    return this.exec(script, 'read');
  }

  async predict(params: PredictParams): Promise<any> {
    const script = this.buildPredictScript(params);
    return this.exec(script, 'read');
  }

  private buildFetchScript(params: FetchParams): string {
    const parts = [
      `'${this.config.readToken || this.config.token || ''}'`,
      `'${params.selector}'`,
      params.labels ? `${this.formatLabelsForScript(params.labels)}` : '{}',
      params.end || 'NOW',
      params.start !== undefined ? String(params.start) : '-1'
    ];

    if (params.count) {
      parts.push(String(params.count));
    }

    return `[ ${parts.join(' ')} ] FETCH`;
  }

  private buildFindScript(params: FindParams): string {
    const parts = [
      `'${this.config.readToken || this.config.token || ''}'`,
      `'${params.selector}'`,
      params.labels ? `${this.formatLabelsForScript(params.labels)}` : '{}'
    ];

    return `[ ${parts.join(' ')} ] FIND`;
  }

  private buildMetaScript(params: MetaParams): string {
    if (params.attributes) {
      // Setting metadata
      const parts = [
        `'${this.config.writeToken || this.config.token || ''}'`,
        `'${params.selector}'`,
        params.labels ? `${this.formatLabelsForScript(params.labels)}` : '{}',
        this.formatAttributesForScript(params.attributes)
      ];

      return `[ ${parts.join(' ')} ] META`;
    } else {
      // Getting metadata
      return this.buildFindScript(params);
    }
  }

  private buildLabelValuesScript(params: LabelValuesParams): string {
    // Build labels with wildcard for the requested label to only get series that have it
    const labels: Record<string, string> = {
      ...params.otherLabels,
      [params.labelName]: '~.*'
    };

    const parts = [
      `'${this.config.readToken || this.config.token || ''}'`,
      `'${params.selector}'`,
      this.formatLabelsForScript(labels)
    ];

    // FIND then extract label values, map, sort and get unique values
    return `[ ${parts.join(' ')} ] FIND
<%
  LABELS '${params.labelName}' GET
%>
false LMAP
LSORT
UNIQUE`;
  }

  private buildPredictScript(params: PredictParams): string {
    // Fetch historical data
    const fetchScript = `
[ '${this.config.readToken || this.config.token || ''}'
  '${params.selector}'
  ${params.labels ? this.formatLabelsForScript(params.labels) : '{}'}
  NOW
  ${params.historyPeriod}
] FETCH
0 GET  // Get first series
'series' STORE
`;

    // Bucketize if needed
    const bucketScript = params.bucketSpan ? `
$series
[ SWAP bucketizer.mean 0 ${params.bucketSpan} 0 ] BUCKETIZE
0 GET  // Get first bucketed series
'series' STORE
` : '';

    // Build prediction script based on method
    let predictionScript = '';

    switch (params.method) {
      case 'holt_winters':
        predictionScript = this.buildHoltWintersScript(params);
        break;
      case 'arima':
        predictionScript = this.buildARIMAScript(params);
        break;
      case 'ses':
        predictionScript = this.buildSESScript(params);
        break;
      case 'linear_regression':
        predictionScript = this.buildLinearRegressionScript(params);
        break;
      case 'polynomial':
        predictionScript = this.buildPolynomialScript(params);
        break;
      case 'fft_forecast':
        predictionScript = this.buildFFTForecastScript(params);
        break;
      case 'lstm':
        predictionScript = this.buildLSTMScript(params);
        break;
      case 'auto':
        predictionScript = this.buildAutoSelectScript(params);
        break;
    }

    return fetchScript + bucketScript + predictionScript;
  }

  private buildHoltWintersScript(params: PredictParams): string {
    const alpha = params.options?.alpha || 0.5;
    const beta = params.options?.beta || 0.5;
    const gamma = params.options?.gamma || 0.5;
    const seasonality = params.options?.seasonality || 24;

    return `
// Holt-Winters prediction
$series
${alpha} ${beta} ${gamma} ${seasonality} ${params.predictionHorizon}
HOLT.WINTERS.FORECAST
'prediction' STORE

// Return both original and prediction
{
  'original' $series
  'predicted' $prediction
  'method' 'holt_winters'
  'parameters' {
    'alpha' ${alpha}
    'beta' ${beta}
    'gamma' ${gamma}
    'seasonality' ${seasonality}
    'horizon' ${params.predictionHorizon}
  }
}`;
  }

  private buildARIMAScript(params: PredictParams): string {
    const p = params.options?.p || 1;
    const d = params.options?.d || 1;
    const q = params.options?.q || 1;

    return `
// ARIMA prediction
$series
${p} ${d} ${q} ${params.predictionHorizon}
ARIMA.FORECAST
'prediction' STORE

{
  'original' $series
  'predicted' $prediction
  'method' 'arima'
  'parameters' {
    'p' ${p}
    'd' ${d}
    'q' ${q}
    'horizon' ${params.predictionHorizon}
  }
}`;
  }

  private buildSESScript(params: PredictParams): string {
    const alpha = params.options?.alpha || 0.5;

    return `
// Simple Exponential Smoothing
$series
${alpha} ${params.predictionHorizon}
SES.FORECAST
'prediction' STORE

{
  'original' $series
  'predicted' $prediction
  'method' 'ses'
  'parameters' {
    'alpha' ${alpha}
    'horizon' ${params.predictionHorizon}
  }
}`;
  }

  private buildLinearRegressionScript(params: PredictParams): string {
    return `
// Linear Regression
$series
VALUEDEDUP  // Remove duplicates
'clean' STORE

// Extract timestamps and values
$clean VALUES 'y' STORE
$clean TICKLIST 'x' STORE

// Perform linear regression
$x $y LREGRESS
'model' STORE

// Generate future timestamps
$clean LASTTICK 'lastTime' STORE
$lastTime ${params.bucketSpan || '1h'} DURATION *
1 ${params.predictionHorizon}
<%
  'i' STORE
  $lastTime $i ${params.bucketSpan || '1h'} DURATION * +
%> FOR
'futureTimestamps' STORE

// Apply model to future timestamps
$futureTimestamps
<%
  't' STORE
  $t $model 0 GET * $model 1 GET +
%> LMAP
'predictions' STORE

// Create prediction GTS
NEWGTS
$clean NAME 'prediction' RENAME
$clean LABELS RELABEL
$futureTimestamps $predictions ZIP
<%
  'point' STORE
  $point 0 GET NaN NaN NaN $point 1 GET ADDVALUE
%> FOREACH
'prediction' STORE

{
  'original' $series
  'predicted' $prediction
  'method' 'linear_regression'
  'model' $model
  'parameters' {
    'horizon' ${params.predictionHorizon}
  }
}`;
  }

  private buildPolynomialScript(params: PredictParams): string {
    const degree = params.options?.degree || 2;

    return `
// Polynomial Regression
$series
${degree} ${params.predictionHorizon}
POLYFIT.FORECAST
'prediction' STORE

{
  'original' $series
  'predicted' $prediction
  'method' 'polynomial'
  'parameters' {
    'degree' ${degree}
    'horizon' ${params.predictionHorizon}
  }
}`;
  }

  private buildFFTForecastScript(params: PredictParams): string {
    return `
// FFT-based Forecast
$series
FFT
'fft' STORE

// Identify dominant frequencies
$fft ABS SORT REVERSE
0 10 SUBLIST  // Top 10 frequencies
'dominantFreq' STORE

// Extrapolate based on dominant frequencies
$series
$dominantFreq
${params.predictionHorizon}
FFT.FORECAST
'prediction' STORE

{
  'original' $series
  'predicted' $prediction
  'method' 'fft_forecast'
  'dominantFrequencies' $dominantFreq
  'parameters' {
    'horizon' ${params.predictionHorizon}
  }
}`;
  }

  private buildLSTMScript(params: PredictParams): string {
    const epochs = params.options?.epochs || 100;
    const hiddenUnits = params.options?.hiddenUnits || 32;

    return `
// LSTM prediction (requires Warp10 ML extension)
$series
{
  'epochs' ${epochs}
  'hidden_units' ${hiddenUnits}
  'horizon' ${params.predictionHorizon}
}
LSTM.FORECAST
'prediction' STORE

{
  'original' $series
  'predicted' $prediction
  'method' 'lstm'
  'parameters' {
    'epochs' ${epochs}
    'hidden_units' ${hiddenUnits}
    'horizon' ${params.predictionHorizon}
  }
}`;
  }

  private buildAutoSelectScript(params: PredictParams): string {
    return `
// Auto-select best method based on data characteristics
$series
'input' STORE

// Check for seasonality
$input SEASONAL.DETECT
'hasSeasonality' STORE

// Check for trend
$input TREND.DETECT
'hasTrend' STORE

// Check data size
$input SIZE 'dataSize' STORE

// Select method based on characteristics
$hasSeasonality $hasTrend AND
<%
  // Use Holt-Winters for seasonal data with trend
  $input 0.5 0.5 0.5 24 ${params.predictionHorizon}
  HOLT.WINTERS.FORECAST
  'prediction' STORE
  'holt_winters' 'selectedMethod' STORE
%>
<%
  $hasTrend
  <%
    // Use linear regression for trending data
    $input ${params.predictionHorizon}
    LREGRESS.FORECAST
    'prediction' STORE
    'linear_regression' 'selectedMethod' STORE
  %>
  <%
    // Use SES for stationary data
    $input 0.5 ${params.predictionHorizon}
    SES.FORECAST
    'prediction' STORE
    'ses' 'selectedMethod' STORE
  %>
  IFTE
%>
IFTE

{
  'original' $input
  'predicted' $prediction
  'method' $selectedMethod
  'analysis' {
    'hasSeasonality' $hasSeasonality
    'hasTrend' $hasTrend
    'dataSize' $dataSize
  }
  'parameters' {
    'horizon' ${params.predictionHorizon}
  }
}`;
  }

  private buildGTSInput(params: UpdateParams): string {
    const lines: string[] = [];

    for (const point of params.dataPoints) {
      const parts = [String(point.timestamp)];

      if (point.latitude !== undefined && point.longitude !== undefined) {
        parts.push(`${point.latitude}:${point.longitude}`);
        if (point.elevation !== undefined) {
          parts[parts.length - 1] += `/${point.elevation}`;
        }
      } else {
        parts.push('/');
      }

      parts.push(`${params.className}${this.formatLabels(params.labels)}`);
      parts.push(this.formatValue(point.value));

      lines.push(parts.join('/'));
    }

    return lines.join('\n');
  }

  private formatLabels(labels?: Record<string, string>): string {
    if (!labels || Object.keys(labels).length === 0) {
      return '{}';
    }

    const labelPairs = Object.entries(labels)
      .map(([key, value]) => `${key}=${value}`)
      .join(',');

    return `{${labelPairs}}`;
  }

  private formatLabelsForScript(labels: Record<string, string>): string {
    const labelPairs = Object.entries(labels)
      .map(([key, value]) => `'${key}' '${value}'`)
      .join(' ');

    return `{ ${labelPairs} }`;
  }

  private formatAttributesForScript(attributes: Record<string, string>): string {
    const attrPairs = Object.entries(attributes)
      .map(([key, value]) => `'${key}' '${value}'`)
      .join(' ');

    return `{ ${attrPairs} }`;
  }

  private formatValue(value: any): string {
    if (typeof value === 'string') {
      return `'${value}'`;
    } else if (typeof value === 'boolean') {
      return value ? 'T' : 'F';
    } else {
      return String(value);
    }
  }
}