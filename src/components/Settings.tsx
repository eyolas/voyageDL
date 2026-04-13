/**
 * Settings - App configuration modal
 */

import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { AppConfig } from '../types';
import { Alert } from './Alert';

interface SettingsProps {
  config: AppConfig;
  onClose: () => void;
  onSave: (config: AppConfig) => Promise<boolean>;
}

export function Settings({ config, onClose, onSave }: SettingsProps) {
  const [cacheMessage, setCacheMessage] = useState<string | null>(null);

  // Close on Escape
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onClose]);

  const handleChangeFolder = useCallback(async () => {
    try {
      const selected = await open({
        directory: true,
        title: 'Selectionner le dossier de telechargement',
      });
      if (selected && typeof selected === 'string') {
        await onSave({ ...config, download_dir: selected });
      }
    } catch (error) {
      console.error('Error selecting folder:', error);
    }
  }, [config, onSave]);

  const handleClearYoutubeCache = async () => {
    try {
      const count = await invoke<number>('clear_youtube_cache');
      setCacheMessage(`Cache YouTube vide (${count} entree${count > 1 ? 's' : ''} supprimee${count > 1 ? 's' : ''})`);
      setTimeout(() => setCacheMessage(null), 3000);
    } catch (err) {
      console.error('Failed to clear YouTube cache:', err);
    }
  };

  const handleClearDeezerCache = async () => {
    try {
      const count = await invoke<number>('clear_deezer_cache');
      setCacheMessage(`Cache Deezer vide (${count} entree${count > 1 ? 's' : ''} supprimee${count > 1 ? 's' : ''})`);
      setTimeout(() => setCacheMessage(null), 3000);
    } catch (err) {
      console.error('Failed to clear Deezer cache:', err);
    }
  };

  return (
    <div className="modal-overlay" onClick={(e) => {
      if (e.target === e.currentTarget) onClose();
    }}>
      <div className="modal">
        <div className="modal-header">
          <h2 className="modal-title">Parametres</h2>
          <button className="modal-close" onClick={onClose}>
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        <div className="modal-content">
          {cacheMessage && (
            <Alert type="info" title="Cache" message={cacheMessage} onClose={() => setCacheMessage(null)} />
          )}

          {/* Download Folder */}
          <div className="form-group">
            <label className="form-label">Dossier de telechargement</label>
            <div className="folder-selection" onClick={handleChangeFolder} style={{ cursor: 'pointer' }}>
              <span className="folder-path">
                {config.download_dir || 'Aucun dossier selectionne'}
              </span>
              <button
                className="button button-secondary"
                style={{ flex: 'none', padding: 'var(--spacing-sm) var(--spacing-md)' }}
              >
                Parcourir
              </button>
            </div>
          </div>

          {/* Cache management */}
          <div className="form-group">
            <label className="form-label">Gestion du cache</label>
            <p className="form-help-text" style={{ marginBottom: 'var(--spacing-md)' }}>
              Le cache stocke les resultats d'analyse pour accelerer les prochaines recherches.
            </p>
            <div className="cache-actions">
              <button className="cache-clear-btn" onClick={handleClearYoutubeCache}>
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <polyline points="3 6 5 6 21 6" />
                  <path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2" />
                </svg>
                Vider le cache YouTube
              </button>
              <button className="cache-clear-btn" onClick={handleClearDeezerCache}>
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <polyline points="3 6 5 6 21 6" />
                  <path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2" />
                </svg>
                Vider le cache Deezer
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
