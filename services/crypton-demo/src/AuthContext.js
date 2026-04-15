import { createContext, useContext } from 'react';

export const AuthContext = createContext({ authUser: null, authReady: false, logout: null });

export const useAuth = () => useContext(AuthContext);
