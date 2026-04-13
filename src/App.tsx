/**
 * Main App component for Voyage DL
 * Manages screen navigation, app state, and download queue
 */

import { useEffect, useState, useCallback } from 'react';
import './App.css';
import './index.css';
import { useConfig } from './hooks/useConfig';
import { SetupScreen } from './components/SetupScreen';
import { MainScreen } from './components/MainScreen';
import { Settings } from './components/Settings';
import { DownloadQueue, DownloadJob } from './components/DownloadQueue';
import { AppConfig, TrackInfo } from './types';

type Screen = 'setup' | 'main';

let jobCounter = 0;

function App() {
  const { config, loading, isConfigured, saveConfig } = useConfig();
  const [currentScreen, setCurrentScreen] = useState<Screen>('setup');
  const [showSettings, setShowSettings] = useState(false);
  const [downloadJobs, setDownloadJobs] = useState<DownloadJob[]>([]);

  useEffect(() => {
    if (!loading) {
      if (isConfigured()) {
        setCurrentScreen('main');
      } else {
        setCurrentScreen('setup');
      }
    }
  }, [loading, isConfigured]);

  const handleSetupComplete = () => {
    setCurrentScreen('main');
  };

  const handleChangeFolder = async (newPath: string) => {
    try {
      await saveConfig({ download_dir: newPath });
    } catch (error) {
      console.error('Error changing folder:', error);
    }
  };

  const handleSettingsSave = async (newConfig: AppConfig): Promise<boolean> => {
    return await saveConfig(newConfig);
  };

  const handleAddToQueue = useCallback((tracks: TrackInfo[]) => {
    jobCounter += 1;
    const job: DownloadJob = {
      id: `job-${jobCounter}-${Date.now()}`,
      tracks,
      outputDir: config.download_dir,
    };
    setDownloadJobs((prev) => [...prev, job]);
  }, [config.download_dir]);

  const handleJobDone = useCallback((_jobId: string) => {
    // Keep the job in the list so the queue can display it as completed
  }, []);

  if (loading) {
    return (
      <div className="app">
        <div className="app-container">
          <div className="screen" style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}>
            <div style={{ textAlign: 'center' }}>
              <div className="equalizer" style={{ justifyContent: 'center', height: '40px', marginBottom: '20px' }}>
                <div className="equalizer-bar" />
                <div className="equalizer-bar" />
                <div className="equalizer-bar" />
                <div className="equalizer-bar" />
                <div className="equalizer-bar" />
              </div>
              <p style={{ fontFamily: 'var(--font-display)', color: 'var(--color-text-secondary)', fontWeight: 500 }}>Chargement...</p>
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="app">
      <div className="app-container">
        {currentScreen === 'setup' && (
          <SetupScreen onSetupComplete={handleSetupComplete} />
        )}

        {currentScreen === 'main' && (
          <MainScreen
            config={config}
            onSettingsClick={() => setShowSettings(true)}
            onChangeFolder={handleChangeFolder}
            onAddToQueue={handleAddToQueue}
          />
        )}
      </div>

      <DownloadQueue jobs={downloadJobs} onJobDone={handleJobDone} />

      {showSettings && (
        <Settings
          config={config}
          onClose={() => setShowSettings(false)}
          onSave={handleSettingsSave}
        />
      )}
    </div>
  );
}

export default App;
