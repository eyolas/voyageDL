/**
 * Custom hook to manage app configuration
 */

import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { AppConfig } from '../types';

const DEFAULT_CONFIG: AppConfig = {
  download_dir: '',
};

export function useConfig() {
  const [config, setConfig] = useState<AppConfig>(DEFAULT_CONFIG);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Load config on mount
  useEffect(() => {
    loadConfig();
  }, []);

  const loadConfig = async () => {
    try {
      setLoading(true);
      setError(null);
      const loadedConfig = await invoke<AppConfig>('get_config');
      setConfig(loadedConfig);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Failed to load config';
      setError(errorMsg);
      console.error('Error loading config:', err);
    } finally {
      setLoading(false);
    }
  };

  const saveConfig = async (newConfig: Partial<AppConfig>) => {
    try {
      setError(null);
      const updatedConfig = { ...config, ...newConfig };
      await invoke('save_config', { config: updatedConfig });
      setConfig(updatedConfig);
      return true;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Failed to save config';
      setError(errorMsg);
      console.error('Error saving config:', err);
      return false;
    }
  };

  const isConfigured = (): boolean => {
    return !!config.download_dir;
  };

  return {
    config,
    loading,
    error,
    saveConfig,
    loadConfig,
    isConfigured,
  };
}
