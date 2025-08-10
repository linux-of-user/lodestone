import React from 'react';
import ModsSearch from './ModsSearch';
import ModsInstalled from './ModsInstalled';

import { useContext } from 'react';
import { InstanceContext } from 'data/InstanceContext';
const ModsTab: React.FC = () => {
  const { selectedInstance } = useContext(InstanceContext);
  const uuid = selectedInstance?.uuid;

  if (!uuid) {
    return <div>No instance selected.</div>;
  }

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