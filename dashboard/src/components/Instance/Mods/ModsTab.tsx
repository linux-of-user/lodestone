import React from 'react';
import ModsSearch from './ModsSearch';
import ModsInstalled from './ModsInstalled';

const ModsTab: React.FC<{ uuid?: string }> = ({ uuid }) => {
  // uuid prop may be injected by tab system or from URL.
  // For now, use a static uuid or inject via props/context as you wire up tabs.
  return (
    <div className="p-4">
      <div className="mb-6">
        <ModsSearch uuid={uuid} />
      </div>
      <ModsInstalled uuid={uuid} />
    </div>
  );
};

export default ModsTab;