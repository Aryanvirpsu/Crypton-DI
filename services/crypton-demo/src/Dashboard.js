import { getAdminToken } from './auth';
import UserDashboard from './UserDashboard';
import AdminDashboard from './AdminDashboard';

export default function Dashboard({ go, toast }) {
  return getAdminToken()
    ? <AdminDashboard go={go} toast={toast} />
    : <UserDashboard go={go} toast={toast} />;
}
